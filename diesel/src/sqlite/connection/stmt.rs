extern crate libsqlite3_sys as ffi;

use super::bind_collector::{SqliteBindCollector, SqliteBindValue};
use super::raw::RawConnection;
use super::sqlite_value::OwnedSqliteValue;
use crate::connection::{MaybeCached, PrepareForCache};
use crate::query_builder::{QueryFragment, QueryId};
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::{Sqlite, SqliteType};
use crate::util::OnceCell;
use std::ffi::{CStr, CString};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::{self, NonNull};

#[allow(missing_debug_implementations)]
pub(in crate::sqlite) struct Statement {
    inner_statement: NonNull<ffi::sqlite3_stmt>,
}

impl Statement {
    pub fn prepare(
        raw_connection: &RawConnection,
        sql: &str,
        is_cached: PrepareForCache,
    ) -> QueryResult<Self> {
        let mut stmt = ptr::null_mut();
        let mut unused_portion = ptr::null();
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
        value: SqliteBindValue<'_>,
        bind_index: i32,
    ) -> QueryResult<Option<NonNull<[u8]>>> {
        let mut ret_ptr = None;
        let result = match (tpe, value) {
            (_, SqliteBindValue::Null) => {
                ffi::sqlite3_bind_null(self.inner_statement.as_ptr(), bind_index)
            }
            (SqliteType::Binary, SqliteBindValue::BorrowedBinary(bytes)) => ffi::sqlite3_bind_blob(
                self.inner_statement.as_ptr(),
                bind_index,
                bytes.as_ptr() as *const libc::c_void,
                bytes.len() as libc::c_int,
                ffi::SQLITE_STATIC(),
            ),
            (SqliteType::Binary, SqliteBindValue::Binary(mut bytes)) => {
                let len = bytes.len();
                // We need a seperate pointer here to pass it to sqlite
                // as the returned pointer is a pointer to a dyn sized **slice**
                // and not the pointer to the first element of the slice
                let ptr;
                ret_ptr = if len > 0 {
                    ptr = bytes.as_mut_ptr();
                    NonNull::new(Box::into_raw(bytes))
                } else {
                    ptr = std::ptr::null_mut();
                    None
                };
                ffi::sqlite3_bind_blob(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    ptr as *const libc::c_void,
                    len as libc::c_int,
                    ffi::SQLITE_STATIC(),
                )
            }
            (SqliteType::Text, SqliteBindValue::BorrowedString(bytes)) => ffi::sqlite3_bind_text(
                self.inner_statement.as_ptr(),
                bind_index,
                bytes.as_ptr() as *const libc::c_char,
                bytes.len() as libc::c_int,
                ffi::SQLITE_STATIC(),
            ),
            (SqliteType::Text, SqliteBindValue::String(bytes)) => {
                let mut bytes = Box::<[u8]>::from(bytes);
                let len = bytes.len();
                // We need a seperate pointer here to pass it to sqlite
                // as the returned pointer is a pointer to a dyn sized **slice**
                // and not the pointer to the first element of the slice
                let ptr;
                ret_ptr = if len > 0 {
                    ptr = bytes.as_mut_ptr();
                    NonNull::new(Box::into_raw(bytes))
                } else {
                    ptr = std::ptr::null_mut();
                    None
                };
                ffi::sqlite3_bind_text(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    ptr as *const libc::c_char,
                    len as libc::c_int,
                    ffi::SQLITE_STATIC(),
                )
            }
            (SqliteType::Float, SqliteBindValue::F64(value))
            | (SqliteType::Double, SqliteBindValue::F64(value)) => ffi::sqlite3_bind_double(
                self.inner_statement.as_ptr(),
                bind_index,
                value as libc::c_double,
            ),
            (SqliteType::SmallInt, SqliteBindValue::I32(value))
            | (SqliteType::Integer, SqliteBindValue::I32(value)) => {
                ffi::sqlite3_bind_int(self.inner_statement.as_ptr(), bind_index, value)
            }
            (SqliteType::Long, SqliteBindValue::I64(value)) => {
                ffi::sqlite3_bind_int64(self.inner_statement.as_ptr(), bind_index, value)
            }
            (t, b) => {
                return Err(Error::SerializationError(
                    format!("Type missmatch: Expected {:?}, got {}", t, b).into(),
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
                    "Error finalizing SQLite prepared statement: {:?}",
                    e
                )
                .expect("Error writing to `stderr`");
            } else {
                panic!("Error finalizing SQLite prepared statement: {:?}", e);
            }
        }
    }
}

// A warning for future editiors:
// Changing this code to something "simplier" may
// introduce undefined behaviour. Make sure you read
// the following discussions for details about
// the current version:
//
// * https://github.com/weiznich/diesel/pull/7
// * https://users.rust-lang.org/t/code-review-for-unsafe-code-in-diesel/66798/
// * https://github.com/rust-lang/unsafe-code-guidelines/issues/194
struct BoundStatement<'stmt, 'query> {
    statement: MaybeCached<'stmt, Statement>,
    // we need to store the query here to ensure noone does
    // drop it till the end ot the statement
    // We use a boxed queryfragment here just to erase the
    // generic type, we use NonNull to communicate
    // that this is a shared buffer
    query: Option<NonNull<dyn QueryFragment<Sqlite> + 'query>>,
    // we need to store any owned bind values speratly, as they are not
    // contained in the query itself. We use NonNull to
    // communicate that this is a shared buffer
    binds_to_free: Vec<(i32, Option<NonNull<[u8]>>)>,
}

impl<'stmt, 'query> BoundStatement<'stmt, 'query> {
    fn bind<T>(
        statement: MaybeCached<'stmt, Statement>,
        query: T,
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
        query.collect_binds(&mut bind_collector, &mut ())?;
        let SqliteBindCollector { binds } = bind_collector;

        let mut ret = BoundStatement {
            statement,
            query: None,
            binds_to_free: Vec::with_capacity(
                binds
                    .iter()
                    .filter(|&(b, _)| {
                        matches!(
                            b,
                            SqliteBindValue::BorrowedBinary(_)
                                | SqliteBindValue::BorrowedString(_)
                                | SqliteBindValue::String(_)
                                | SqliteBindValue::Binary(_)
                        )
                    })
                    .count(),
            ),
        };

        ret.bind_buffers(binds)?;

        let query = query as Box<dyn QueryFragment<Sqlite> + 'query>;
        ret.query = NonNull::new(Box::into_raw(query));

        Ok(ret)
    }

    // This is a seperate function so that
    // not the whole construtor is generic over the query type T.
    // This hopefully prevents binary bloat.
    fn bind_buffers(&mut self, binds: Vec<(SqliteBindValue<'_>, SqliteType)>) -> QueryResult<()> {
        for (bind_idx, (bind, tpe)) in (1..).zip(binds) {
            if matches!(
                bind,
                SqliteBindValue::BorrowedString(_) | SqliteBindValue::BorrowedBinary(_)
            ) {
                // Store the id's of borrowed binds to unbind them on drop
                self.binds_to_free.push((bind_idx, None));
            }

            // It's safe to call bind here as:
            // * The type and value matches
            // * We ensure that corresponding buffers lives long enough below
            // * The statement is not used yet by `step` or anything else
            let res = unsafe { self.statement.bind(tpe, bind, bind_idx) }?;
            if let Some(ptr) = res {
                // Store the id + pointer for a owned bind
                // as we must unbind and free them on drop
                self.binds_to_free.push((bind_idx, Some(ptr)));
            }
        }
        Ok(())
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
                    .bind(SqliteType::Text, SqliteBindValue::Null, idx)
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
                    // got the pointer from a box + it is guarenteed to be not null.
                    std::mem::drop(Box::from_raw(buffer.as_ptr()));
                }
            }
        }

        if let Some(query) = self.query {
            unsafe {
                // Constructing the `Box` here is safe as we
                // got the pointer from a box + it is guarenteed to be not null.
                std::mem::drop(Box::from_raw(query.as_ptr()));
            }
            self.query = None;
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct StatementUse<'stmt, 'query> {
    statement: BoundStatement<'stmt, 'query>,
    column_names: OnceCell<Vec<*const str>>,
    called_step_once: bool,
}

impl<'stmt, 'query> StatementUse<'stmt, 'query> {
    pub(super) fn bind<T>(
        statement: MaybeCached<'stmt, Statement>,
        query: T,
    ) -> QueryResult<StatementUse<'stmt, 'query>>
    where
        T: QueryFragment<Sqlite> + QueryId + 'query,
    {
        Ok(Self {
            statement: BoundStatement::bind(statement, query)?,
            column_names: OnceCell::new(),
            called_step_once: false,
        })
    }

    pub(in crate::sqlite::connection) fn run(self) -> QueryResult<()> {
        self.step().map(|_| ())
    }

    pub(in crate::sqlite::connection) fn step(self) -> QueryResult<Option<Self>> {
        let res = unsafe {
            match ffi::sqlite3_step(self.statement.statement.inner_statement.as_ptr()) {
                ffi::SQLITE_DONE => Ok(None),
                ffi::SQLITE_ROW => Ok(Some(())),
                _ => Err(last_error(self.statement.statement.raw_connection())),
            }
        }?;
        Ok(res.map(move |()| {
            if self.called_step_once {
                self
            } else {
                Self {
                    called_step_once: true,
                    column_names: OnceCell::new(),
                    ..self
                }
            }
        }))
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
             horribliy wrong. Please open an issue at the \
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
