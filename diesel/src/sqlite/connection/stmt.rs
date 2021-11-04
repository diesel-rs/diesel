extern crate libsqlite3_sys as ffi;

use super::raw::RawConnection;
use super::sqlite_value::OwnedSqliteValue;
use crate::connection::{MaybeCached, PrepareForCache};
use crate::query_builder::{QueryFragment, QueryId};
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::query_builder::{SqliteBindCollector, SqliteBindValue};
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

    unsafe fn bind(
        &mut self,
        tpe: SqliteType,
        value: &SqliteBindValue,
        bind_index: i32,
    ) -> QueryResult<()> {
        // This unsafe block assumes the following invariants:
        //
        // - `stmt` points to valid memory
        // - If `self.ty` is anything other than `Binary` or `Text`, the appropriate
        //   number of bytes were written to `value` for an integer of the
        //   corresponding size.
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
            (SqliteType::Binary, SqliteBindValue::Binary(bytes)) => ffi::sqlite3_bind_blob(
                self.inner_statement.as_ptr(),
                bind_index,
                bytes.as_ptr() as *const libc::c_void,
                bytes.len() as libc::c_int,
                ffi::SQLITE_STATIC(),
            ),
            (SqliteType::Text, SqliteBindValue::BorrowedString(bytes)) => ffi::sqlite3_bind_text(
                self.inner_statement.as_ptr(),
                bind_index,
                bytes.as_ptr() as *const libc::c_char,
                bytes.len() as libc::c_int,
                ffi::SQLITE_STATIC(),
            ),
            (SqliteType::Text, SqliteBindValue::String(bytes)) => ffi::sqlite3_bind_text(
                self.inner_statement.as_ptr(),
                bind_index,
                bytes.as_ptr() as *const libc::c_char,
                bytes.len() as libc::c_int,
                ffi::SQLITE_STATIC(),
            ),
            (SqliteType::Float, SqliteBindValue::Float(value)) => ffi::sqlite3_bind_double(
                self.inner_statement.as_ptr(),
                bind_index,
                libc::c_double::from(*value),
            ),
            (SqliteType::Double, SqliteBindValue::Double(value)) => ffi::sqlite3_bind_double(
                self.inner_statement.as_ptr(),
                bind_index,
                *value as libc::c_double,
            ),
            (SqliteType::SmallInt, SqliteBindValue::SmallInt(value)) => ffi::sqlite3_bind_int(
                self.inner_statement.as_ptr(),
                bind_index,
                libc::c_int::from(*value),
            ),
            (SqliteType::Integer, SqliteBindValue::Integer(value)) => {
                ffi::sqlite3_bind_int(self.inner_statement.as_ptr(), bind_index, *value)
            }
            (SqliteType::Long, SqliteBindValue::BigInt(value)) => {
                ffi::sqlite3_bind_int64(self.inner_statement.as_ptr(), bind_index, *value)
            }
            (t, b) => {
                return Err(Error::DeserializationError(
                    format!("Type missmatch: Expected {:?}, got {}", t, b).into(),
                ))
            }
        };
        ensure_sqlite_ok(result, self.raw_connection())
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

#[allow(missing_debug_implementations)]
pub struct StatementUse<'a, 'b> {
    statement: MaybeCached<'a, Statement>,
    column_names: OnceCell<Vec<*const str>>,
    // we need to store the query here to ensure noone does
    // drop it till the end ot the statement
    // We use a boxed queryfragment here just to erase the
    // generic type
    _query: Box<dyn QueryFragment<Sqlite> + 'b>,
    binds_to_free: Vec<(i32, Option<SqliteBindValue<'static>>)>,
}

impl<'a, 'b> StatementUse<'a, 'b> {
    pub(super) fn bind<T>(mut statement: MaybeCached<'a, Statement>, query: T) -> QueryResult<Self>
    where
        T: QueryFragment<Sqlite> + QueryId + 'b,
    {
        let mut bind_collector = SqliteBindCollector::new();
        query.collect_binds(&mut bind_collector, &mut ())?;

        let SqliteBindCollector { metadata, binds } = bind_collector;
        let mut binds_to_free = Vec::new();

        for (idx, (bind, tpe)) in binds.into_iter().zip(metadata).enumerate() {
            // Sqlite starts to count by 1
            let bind_idx = idx as i32 + 1;

            // It's safe to call bind here as:
            // * The type and value matches
            // * We ensure that corresponding buffers lives long enough below
            // * The statement is not used yet by `step` or anything else
            unsafe { statement.bind(tpe, &bind, bind_idx)? };

            // We want to unbind the buffers later to ensure
            // that sqlite does not access uninitilized memory
            match bind {
                SqliteBindValue::BorrowedString(_) | SqliteBindValue::BorrowedBinary(_) => {
                    binds_to_free.push((bind_idx, None));
                }
                SqliteBindValue::Binary(b) => {
                    binds_to_free.push((bind_idx, Some(SqliteBindValue::Binary(b))));
                }
                SqliteBindValue::String(b) => {
                    binds_to_free.push((bind_idx, Some(SqliteBindValue::String(b))));
                }
                _ => (),
            }
        }

        Ok(Self {
            statement,
            column_names: OnceCell::new(),
            binds_to_free,
            _query: Box::new(query) as Box<_>,
        })
    }

    pub(in crate::sqlite::connection) fn run(self) -> QueryResult<()> {
        self.step().map(|_| ())
    }

    pub(in crate::sqlite::connection) fn step(self) -> QueryResult<Option<Self>> {
        let res = unsafe {
            match ffi::sqlite3_step(self.statement.inner_statement.as_ptr()) {
                ffi::SQLITE_DONE => Ok(None),
                ffi::SQLITE_ROW => Ok(Some(())),
                _ => Err(last_error(self.statement.raw_connection())),
            }
        }?;
        Ok(res.map(move |()| self))
    }

    // The returned string pointer is valid until either the prepared statement is
    // destroyed by sqlite3_finalize() or until the statement is automatically
    // reprepared by the first call to sqlite3_step() for a particular run or
    // until the next call to sqlite3_column_name() or sqlite3_column_name16()
    // on the same column.
    //
    // https://sqlite.org/c3ref/column_name.html
    pub(super) unsafe fn column_name(&self, idx: i32) -> *const str {
        let name = {
            let column_name =
                ffi::sqlite3_column_name(self.statement.inner_statement.as_ptr(), idx);
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
        unsafe { ffi::sqlite3_column_count(self.statement.inner_statement.as_ptr()) }
    }

    pub(super) fn index_for_column_name(&mut self, field_name: &str) -> Option<usize> {
        (0..self.column_count())
            .find(|idx| self.field_name(*idx) == Some(field_name))
            .map(|v| v as usize)
    }

    pub(super) fn field_name(&mut self, idx: i32) -> Option<&str> {
        if let Some(column_names) = self.column_names.get() {
            return column_names
                .get(idx as usize)
                .and_then(|c| unsafe { c.as_ref() });
        }
        let values = (0..self.column_count())
            .map(|idx| unsafe { self.column_name(idx) })
            .collect::<Vec<_>>();
        let ret = values.get(idx as usize).copied();
        let _ = self.column_names.set(values);
        ret.and_then(|p| unsafe { p.as_ref() })
    }

    pub(super) fn copy_value(&self, idx: i32) -> Option<OwnedSqliteValue> {
        OwnedSqliteValue::copy_from_ptr(self.column_value(idx)?)
    }

    pub(super) fn column_value(&self, idx: i32) -> Option<NonNull<ffi::sqlite3_value>> {
        let ptr =
            unsafe { ffi::sqlite3_column_value(self.statement.inner_statement.as_ptr(), idx) };
        NonNull::new(ptr)
    }
}

impl<'a, 'b> Drop for StatementUse<'a, 'b> {
    fn drop(&mut self) {
        // First reset the statement, otherwise the bind calls
        // below will fails
        self.statement.reset();

        // Reset the binds that may point to memory that will be/needs to be freed
        for (idx, _buffer) in std::mem::take(&mut self.binds_to_free) {
            unsafe {
                // It's always safe to bind null values
                self.statement
                    .bind(SqliteType::Text, &SqliteBindValue::Null, idx)
                    .expect("Binding nulls shouldn't ever fail");
            }
        }
    }
}
