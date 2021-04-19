extern crate libsqlite3_sys as ffi;

use std::ffi::{CStr, CString};
use std::io::{stderr, Write};
use std::os::raw as libc;
use std::ptr::{self, NonNull};

use super::raw::RawConnection;
use super::serialized_value::SerializedValue;
use super::sqlite_value::SqliteRow;
use super::SqliteValue;
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::SqliteType;

pub struct Statement {
    inner_statement: NonNull<ffi::sqlite3_stmt>,
    bind_index: libc::c_int,
}

impl Statement {
    pub fn prepare(raw_connection: &RawConnection, sql: &str) -> QueryResult<Self> {
        let mut stmt = ptr::null_mut();
        let mut unused_portion = ptr::null();
        let prepare_result = unsafe {
            ffi::sqlite3_prepare_v2(
                raw_connection.internal_connection.as_ptr(),
                CString::new(sql)?.as_ptr(),
                sql.len() as libc::c_int,
                &mut stmt,
                &mut unused_portion,
            )
        };

        ensure_sqlite_ok(prepare_result, raw_connection.internal_connection.as_ptr()).map(|_| {
            Statement {
                inner_statement: unsafe { NonNull::new_unchecked(stmt) },
                bind_index: 0,
            }
        })
    }

    pub fn bind(&mut self, tpe: SqliteType, value: Option<Vec<u8>>) -> QueryResult<()> {
        self.bind_index += 1;
        let value = SerializedValue {
            ty: tpe,
            data: value,
        };
        let result = value.bind_to(self.inner_statement, self.bind_index);

        ensure_sqlite_ok(result, self.raw_connection())
    }

    fn reset(&mut self) {
        self.bind_index = 0;
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

pub struct StatementUse<'a: 'b, 'b> {
    statement: &'a mut Statement,
    column_names: Vec<&'b str>,
    should_init_column_names: bool,
}

impl<'a, 'b> StatementUse<'a, 'b> {
    pub(in crate::sqlite::connection) fn new(
        statement: &'a mut Statement,
        should_init_column_names: bool,
    ) -> Self {
        StatementUse {
            statement,
            // Init with empty vector because column names
            // can change till the first call to `step()`
            column_names: Vec::new(),
            should_init_column_names,
        }
    }

    pub(in crate::sqlite::connection) fn run(&mut self) -> QueryResult<()> {
        self.step().map(|_| ())
    }

    pub(in crate::sqlite::connection) fn step<'c>(
        &'c mut self,
    ) -> QueryResult<Option<SqliteRow<'a, 'b, 'c>>>
    where
        'b: 'c,
    {
        let res = unsafe {
            match ffi::sqlite3_step(self.statement.inner_statement.as_ptr()) {
                ffi::SQLITE_DONE => Ok(None),
                ffi::SQLITE_ROW => Ok(Some(())),
                _ => Err(last_error(self.statement.raw_connection())),
            }
        }?;
        if self.should_init_column_names {
            self.column_names = (0..self.column_count())
                .map(|idx| unsafe { self.column_name(idx) })
                .collect();
            self.should_init_column_names = false;
        }
        Ok(res.map(move |()| SqliteRow::new(self)))
    }

    // The returned string pointer is valid until either the prepared statement is
    // destroyed by sqlite3_finalize() or until the statement is automatically
    // reprepared by the first call to sqlite3_step() for a particular run or
    // until the next call to sqlite3_column_name() or sqlite3_column_name16()
    // on the same column.
    //
    // https://sqlite.org/c3ref/column_name.html
    //
    // As result of this requirements: Never use that function outside of `ColumnInformation`
    // and never use `ColumnInformation` outside of `StatementUse`
    unsafe fn column_name(&mut self, idx: i32) -> &'b str {
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
        )
    }

    pub(in crate::sqlite::connection) fn column_count(&self) -> i32 {
        unsafe { ffi::sqlite3_column_count(self.statement.inner_statement.as_ptr()) }
    }

    pub(in crate::sqlite::connection) fn index_for_column_name(
        &self,
        field_name: &str,
    ) -> Option<usize> {
        self.column_names
            .iter()
            .enumerate()
            .find(|(_, name)| name == &&field_name)
            .map(|(idx, _)| idx)
    }

    pub(in crate::sqlite::connection) fn field_name<'c>(&'c self, idx: i32) -> Option<&'c str>
    where
        'b: 'c,
    {
        self.column_names.get(idx as usize).copied()
    }

    pub(in crate::sqlite::connection) fn value<'c>(
        &'c self,
        idx: i32,
    ) -> Option<super::SqliteValue<'c>>
    where
        'b: 'c,
    {
        unsafe {
            let ptr = ffi::sqlite3_column_value(self.statement.inner_statement.as_ptr(), idx);
            SqliteValue::new(ptr)
        }
    }
}

impl<'a, 'b> Drop for StatementUse<'a, 'b> {
    fn drop(&mut self) {
        self.statement.reset();
    }
}
