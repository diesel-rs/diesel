#![allow(unsafe_code)] // fii code
use super::bind_collector::{InternalSqliteBindValue, SqliteBindCollector};
use super::raw::RawConnection;
use super::sqlite_value::OwnedSqliteValue;
use crate::connection::statement_cache::{MaybeCached, PrepareForCache};
use crate::connection::Instrumentation;
use crate::query_builder::{QueryFragment, QueryId};
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::{Sqlite, SqliteType};
use libsqlite3_sys as ffi;
use std::cell::OnceCell;
use std::ffi::{CStr, CString};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::{self, NonNull};

pub(super) struct Statement {
    inner_statement: NonNull<ffi::sqlite3_stmt>,
}

impl Statement {
    pub(super) fn prepare(
        raw_connection: &RawConnection,
        sql: &str,
        is_cached: PrepareForCache,
    ) -> QueryResult<Self> {
        let mut stmt = ptr::null_mut();
        let mut unused_portion = ptr::null();
        // the cast for `ffi::SQLITE_PREPARE_PERSISTENT` is required for old libsqlite3-sys versions
        #[allow(clippy::unnecessary_cast)]
        let prepare_result = unsafe {
            ffi::sqlite3_prepare_v3(
                raw_connection.internal_connection.as_ptr(),
                CString::new(sql)?.as_ptr(),
                sql.len() as libc::c_int,
                if matches!(is_cached, PrepareForCache::Yes) {
                    ffi::SQLITE_PREPARE_PERSISTENT as u32
                } else {
                    0
                },
                &mut stmt,
                &mut unused_portion,
            )
        };

        ensure_sqlite_ok(prepare_result, raw_connection.internal_connection.as_ptr()).map(|_| {
            Statement {
                inner_statement: unsafe { NonNull::new_unchecked(stmt) },
            }
        })
    }

    // The caller of this function has to ensure that:
    // * Any buffer provided as `SqliteBindValue::BorrowedBinary`, `SqliteBindValue::Binary`
    // `SqliteBindValue::String` or `SqliteBindValue::BorrowedString` is valid
    // till either a new value is bound to the same parameter or the underlying
    // prepared statement is dropped.
    unsafe fn bind(
        &mut self,
        tpe: SqliteType,
        value: InternalSqliteBindValue<'_>,
        bind_index: i32,
    ) -> QueryResult<Option<NonNull<[u8]>>> {
        let mut ret_ptr = None;
        let result = match (tpe, value) {
            (_, InternalSqliteBindValue::Null) => {
                ffi::sqlite3_bind_null(self.inner_statement.as_ptr(), bind_index)
            }
            (SqliteType::Binary, InternalSqliteBindValue::BorrowedBinary(bytes)) => {
                ffi::sqlite3_bind_blob(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    bytes.as_ptr() as *const libc::c_void,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_STATIC(),
                )
            }
            (SqliteType::Binary, InternalSqliteBindValue::Binary(mut bytes)) => {
                let len = bytes.len();
                // We need a separate pointer here to pass it to sqlite
                // as the returned pointer is a pointer to a dyn sized **slice**
                // and not the pointer to the first element of the slice
                let ptr = bytes.as_mut_ptr();
                ret_ptr = NonNull::new(Box::into_raw(bytes));
                ffi::sqlite3_bind_blob(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    ptr as *const libc::c_void,
                    len as libc::c_int,
                    ffi::SQLITE_STATIC(),
                )
            }
            (SqliteType::Text, InternalSqliteBindValue::BorrowedString(bytes)) => {
                ffi::sqlite3_bind_text(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    bytes.as_ptr() as *const libc::c_char,
                    bytes.len() as libc::c_int,
                    ffi::SQLITE_STATIC(),
                )
            }
            (SqliteType::Text, InternalSqliteBindValue::String(bytes)) => {
                let mut bytes = Box::<[u8]>::from(bytes);
                let len = bytes.len();
                // We need a separate pointer here to pass it to sqlite
                // as the returned pointer is a pointer to a dyn sized **slice**
                // and not the pointer to the first element of the slice
                let ptr = bytes.as_mut_ptr();
                ret_ptr = NonNull::new(Box::into_raw(bytes));
                ffi::sqlite3_bind_text(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    ptr as *const libc::c_char,
                    len as libc::c_int,
                    ffi::SQLITE_STATIC(),
                )
            }
            (SqliteType::Float, InternalSqliteBindValue::F64(value))
            | (SqliteType::Double, InternalSqliteBindValue::F64(value)) => {
                ffi::sqlite3_bind_double(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    value as libc::c_double,
                )
            }
            (SqliteType::SmallInt, InternalSqliteBindValue::I32(value))
            | (SqliteType::Integer, InternalSqliteBindValue::I32(value)) => {
                ffi::sqlite3_bind_int(self.inner_statement.as_ptr(), bind_index, value)
            }
            (SqliteType::Long, InternalSqliteBindValue::I64(value)) => {
                ffi::sqlite3_bind_int64(self.inner_statement.as_ptr(), bind_index, value)
            }
            (t, b) => {
                return Err(Error::SerializationError(
                    format!("Type mismatch: Expected {t:?}, got {b}").into(),
                ))
            }
        };
        match ensure_sqlite_ok(result, self.raw_connection()) {
            Ok(()) => Ok(ret_ptr),
            Err(e) => {
                if let Some(ptr) = ret_ptr {
                    // This is a `NonNul` ptr so it cannot be null
                    // It points to a slice internally as we did not apply
                    // any cast above.
                    std::mem::drop(Box::from_raw(ptr.as_ptr()))
                }
                Err(e)
            }
        }
    }

    fn reset(&mut self) {
        unsafe { ffi::sqlite3_reset(self.inner_statement.as_ptr()) };
    }

    fn raw_connection(&self) -> *mut ffi::sqlite3 {
        unsafe { ffi::sqlite3_db_handle(self.inner_statement.as_ptr()) }
    }
}

pub(super) fn ensure_sqlite_ok(
    code: libc::c_int,
    raw_connection: *mut ffi::sqlite3,
) -> QueryResult<()> {
    if code == ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(last_error(raw_connection))
    }
}

fn last_error(raw_connection: *mut ffi::sqlite3) -> Error {
    let error_message = last_error_message(raw_connection);
    let error_information = Box::new(error_message);
    let error_kind = match last_error_code(raw_connection) {
        ffi::SQLITE_CONSTRAINT_UNIQUE | ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
            DatabaseErrorKind::UniqueViolation
        }
        ffi::SQLITE_CONSTRAINT_FOREIGNKEY => DatabaseErrorKind::ForeignKeyViolation,
        ffi::SQLITE_CONSTRAINT_NOTNULL => DatabaseErrorKind::NotNullViolation,
        ffi::SQLITE_CONSTRAINT_CHECK => DatabaseErrorKind::CheckViolation,
        _ => DatabaseErrorKind::Unknown,
    };
    DatabaseError(error_kind, error_information)
}

fn last_error_message(conn: *mut ffi::sqlite3) -> String {
    let c_str = unsafe { CStr::from_ptr(ffi::sqlite3_errmsg(conn)) };
    c_str.to_string_lossy().into_owned()
}

fn last_error_code(conn: *mut ffi::sqlite3) -> libc::c_int {
    unsafe { ffi::sqlite3_extended_errcode(conn) }
}

impl Drop for Statement {
    fn drop(&mut self) {
        use std::thread::panicking;

        let raw_connection = self.raw_connection();
        let finalize_result = unsafe { ffi::sqlite3_finalize(self.inner_statement.as_ptr()) };
        if let Err(e) = ensure_sqlite_ok(finalize_result, raw_connection) {
            if panicking() {
                write!(
                    stderr(),
                    "Error finalizing SQLite prepared statement: {e:?}"
                )
                .expect("Error writing to `stderr`");
            } else {
                panic!("Error finalizing SQLite prepared statement: {:?}", e);
            }
        }
    }
}

// A warning for future editors:
// Changing this code to something "simpler" may
// introduce undefined behaviour. Make sure you read
// the following discussions for details about
// the current version:
//
// * https://github.com/weiznich/diesel/pull/7
// * https://users.rust-lang.org/t/code-review-for-unsafe-code-in-diesel/66798/
// * https://github.com/rust-lang/unsafe-code-guidelines/issues/194
struct BoundStatement<'stmt, 'query> {
    statement: MaybeCached<'stmt, Statement>,
    // we need to store the query here to ensure no one does
    // drop it till the end of the statement
    // We use a boxed queryfragment here just to erase the
    // generic type, we use NonNull to communicate
    // that this is a shared buffer
    query: Option<NonNull<dyn QueryFragment<Sqlite> + 'query>>,
    // we need to store any owned bind values separately, as they are not
    // contained in the query itself. We use NonNull to
    // communicate that this is a shared buffer
    binds_to_free: Vec<(i32, Option<NonNull<[u8]>>)>,
    instrumentation: &'stmt mut dyn Instrumentation,
    has_error: bool,
}

impl<'stmt, 'query> BoundStatement<'stmt, 'query> {
    fn bind<T>(
        statement: MaybeCached<'stmt, Statement>,
        query: T,
        instrumentation: &'stmt mut dyn Instrumentation,
    ) -> QueryResult<BoundStatement<'stmt, 'query>>
    where
        T: QueryFragment<Sqlite> + QueryId + 'query,
    {
        // Don't use a trait object here to prevent using a virtual function call
        // For sqlite this can introduce a measurable overhead
        // Query is boxed here to make sure it won't move in memory anymore, so any bind
        // it could output would stay valid.
        let query = Box::new(query);

        let mut bind_collector = SqliteBindCollector::new();
        query.collect_binds(&mut bind_collector, &mut (), &Sqlite)?;
        let SqliteBindCollector { binds } = bind_collector;

        let mut ret = BoundStatement {
            statement,
            query: None,
            binds_to_free: Vec::new(),
            instrumentation,
            has_error: false,
        };

        ret.bind_buffers(binds)?;

        let query = query as Box<dyn QueryFragment<Sqlite> + 'query>;
        ret.query = NonNull::new(Box::into_raw(query));

        Ok(ret)
    }

    // This is a separated function so that
    // not the whole constructor is generic over the query type T.
    // This hopefully prevents binary bloat.
    fn bind_buffers(
        &mut self,
        binds: Vec<(InternalSqliteBindValue<'_>, SqliteType)>,
    ) -> QueryResult<()> {
        // It is useful to preallocate `binds_to_free` because it
        // - Guarantees that pushing inside it cannot panic, which guarantees the `Drop`
        //   impl of `BoundStatement` will always re-`bind` as needed
        // - Avoids reallocations
        self.binds_to_free.reserve(
            binds
                .iter()
                .filter(|&(b, _)| {
                    matches!(
                        b,
                        InternalSqliteBindValue::BorrowedBinary(_)
                            | InternalSqliteBindValue::BorrowedString(_)
                            | InternalSqliteBindValue::String(_)
                            | InternalSqliteBindValue::Binary(_)
                    )
                })
                .count(),
        );
        for (bind_idx, (bind, tpe)) in (1..).zip(binds) {
            let is_borrowed_bind = matches!(
                bind,
                InternalSqliteBindValue::BorrowedString(_)
                    | InternalSqliteBindValue::BorrowedBinary(_)
            );

            // It's safe to call bind here as:
            // * The type and value matches
            // * We ensure that corresponding buffers lives long enough below
            // * The statement is not used yet by `step` or anything else
            let res = unsafe { self.statement.bind(tpe, bind, bind_idx) }?;

            // it's important to push these only after
            // the call to bind succeeded, otherwise we might attempt to
            // call bind to an non-existing bind position in
            // the destructor
            if let Some(ptr) = res {
                // Store the id + pointer for a owned bind
                // as we must unbind and free them on drop
                self.binds_to_free.push((bind_idx, Some(ptr)));
            } else if is_borrowed_bind {
                // Store the id's of borrowed binds to unbind them on drop
                self.binds_to_free.push((bind_idx, None));
            }
        }
        Ok(())
    }

    fn finish_query_with_error(mut self, e: &Error) {
        self.has_error = true;
        if let Some(q) = self.query {
            // it's safe to get a reference from this ptr as it's guaranteed to not be null
            let q = unsafe { q.as_ref() };
            self.instrumentation.on_connection_event(
                crate::connection::InstrumentationEvent::FinishQuery {
                    query: &crate::debug_query(&q),
                    error: Some(e),
                },
            );
        }
    }
}

impl<'stmt, 'query> Drop for BoundStatement<'stmt, 'query> {
    fn drop(&mut self) {
        // First reset the statement, otherwise the bind calls
        // below will fails
        self.statement.reset();

        for (idx, buffer) in std::mem::take(&mut self.binds_to_free) {
            unsafe {
                // It's always safe to bind null values, as there is no buffer that needs to outlife something
                self.statement
                    .bind(SqliteType::Text, InternalSqliteBindValue::Null, idx)
                    .expect(
                        "Binding a null value should never fail. \
                             If you ever see this error message please open \
                             an issue at diesels issue tracker containing \
                             code how to trigger this message.",
                    );
            }

            if let Some(buffer) = buffer {
                unsafe {
                    // Constructing the `Box` here is safe as we
                    // got the pointer from a box + it is guaranteed to be not null.
                    std::mem::drop(Box::from_raw(buffer.as_ptr()));
                }
            }
        }

        if let Some(query) = self.query {
            let query = unsafe {
                // Constructing the `Box` here is safe as we
                // got the pointer from a box + it is guaranteed to be not null.
                Box::from_raw(query.as_ptr())
            };
            if !self.has_error {
                self.instrumentation.on_connection_event(
                    crate::connection::InstrumentationEvent::FinishQuery {
                        query: &crate::debug_query(&query),
                        error: None,
                    },
                );
            }
            std::mem::drop(query);
            self.query = None;
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct StatementUse<'stmt, 'query> {
    statement: BoundStatement<'stmt, 'query>,
    column_names: OnceCell<Vec<*const str>>,
}

impl<'stmt, 'query> StatementUse<'stmt, 'query> {
    pub(super) fn bind<T>(
        statement: MaybeCached<'stmt, Statement>,
        query: T,
        instrumentation: &'stmt mut dyn Instrumentation,
    ) -> QueryResult<StatementUse<'stmt, 'query>>
    where
        T: QueryFragment<Sqlite> + QueryId + 'query,
    {
        Ok(Self {
            statement: BoundStatement::bind(statement, query, instrumentation)?,
            column_names: OnceCell::new(),
        })
    }

    pub(super) fn run(mut self) -> QueryResult<()> {
        let r = unsafe {
            // This is safe as we pass `first_step = true`
            // and we consume the statement so nobody could
            // access the columns later on anyway.
            self.step(true).map(|_| ())
        };
        if let Err(ref e) = r {
            self.statement.finish_query_with_error(e);
        }
        r
    }

    // This function is marked as unsafe incorrectly passing `false` to `first_step`
    // for a first call to this function could cause access to freed memory via
    // the cached column names.
    //
    // It's always safe to call this function with `first_step = true` as this removes
    // the cached column names
    pub(super) unsafe fn step(&mut self, first_step: bool) -> QueryResult<bool> {
        let res = match ffi::sqlite3_step(self.statement.statement.inner_statement.as_ptr()) {
            ffi::SQLITE_DONE => Ok(false),
            ffi::SQLITE_ROW => Ok(true),
            _ => Err(last_error(self.statement.statement.raw_connection())),
        };
        if first_step {
            self.column_names = OnceCell::new();
        }
        res
    }

    // The returned string pointer is valid until either the prepared statement is
    // destroyed by sqlite3_finalize() or until the statement is automatically
    // reprepared by the first call to sqlite3_step() for a particular run or
    // until the next call to sqlite3_column_name() or sqlite3_column_name16()
    // on the same column.
    //
    // https://sqlite.org/c3ref/column_name.html
    //
    // Note: This function is marked as unsafe, as calling it can invalidate
    // other existing column name pointers on the same column. To prevent that,
    // it should maximally be called once per column at all.
    unsafe fn column_name(&self, idx: i32) -> *const str {
        let name = {
            let column_name =
                ffi::sqlite3_column_name(self.statement.statement.inner_statement.as_ptr(), idx);
            assert!(
                !column_name.is_null(),
                "The Sqlite documentation states that it only returns a \
                 null pointer here if we are in a OOM condition."
            );
            CStr::from_ptr(column_name)
        };
        name.to_str().expect(
            "The Sqlite documentation states that this is UTF8. \
             If you see this error message something has gone \
             horribly wrong. Please open an issue at the \
             diesel repository.",
        ) as *const str
    }

    pub(super) fn column_count(&self) -> i32 {
        unsafe { ffi::sqlite3_column_count(self.statement.statement.inner_statement.as_ptr()) }
    }

    pub(super) fn index_for_column_name(&mut self, field_name: &str) -> Option<usize> {
        (0..self.column_count())
            .find(|idx| self.field_name(*idx) == Some(field_name))
            .map(|v| v as usize)
    }

    pub(super) fn field_name(&self, idx: i32) -> Option<&str> {
        let column_names = self.column_names.get_or_init(|| {
            let count = self.column_count();
            (0..count)
                .map(|idx| unsafe {
                    // By initializing the whole vec at once we ensure that
                    // we really call this only once.
                    self.column_name(idx)
                })
                .collect()
        });

        column_names
            .get(idx as usize)
            .and_then(|c| unsafe { c.as_ref() })
    }

    pub(super) fn copy_value(&self, idx: i32) -> Option<OwnedSqliteValue> {
        OwnedSqliteValue::copy_from_ptr(self.column_value(idx)?)
    }

    pub(super) fn column_value(&self, idx: i32) -> Option<NonNull<ffi::sqlite3_value>> {
        let ptr = unsafe {
            ffi::sqlite3_column_value(self.statement.statement.inner_statement.as_ptr(), idx)
        };
        NonNull::new(ptr)
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::sql_types::Text;

    // this is a regression test for
    // https://github.com/diesel-rs/diesel/issues/3558
    #[test]
    fn check_out_of_bounds_bind_does_not_panic_on_drop() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();

        let e = crate::sql_query("SELECT '?'")
            .bind::<Text, _>("foo")
            .execute(&mut conn);

        assert!(e.is_err());
        let e = e.unwrap_err();
        if let crate::result::Error::DatabaseError(crate::result::DatabaseErrorKind::Unknown, m) = e
        {
            assert_eq!(m.message(), "column index out of range");
        } else {
            panic!("Wrong error returned");
        }
    }
}
