#![allow(unsafe_code)] // fii code
use super::bind_collector::{SqliteBindCollector, SqliteBindValueRef};
use super::error_information::SqliteErrorInformation;
use super::raw::{ControlStatement, RawConnection};
use super::sqlite_value::OwnedSqliteValue;
use crate::connection::statement_cache::{MaybeCached, PrepareForCache, PreparedStatement};
use crate::connection::{AnsiTransactionManager, Instrumentation};
use crate::query_builder::{QueryFragment, QueryId};
use crate::result::Error::DatabaseError;
use crate::result::*;
use crate::sqlite::{Sqlite, SqliteType};
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::OnceCell;
use core::ffi as libc;
use core::ffi::CStr;
use core::ptr::{self, NonNull};
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use libsqlite3_sys as ffi;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

pub(super) struct Statement {
    inner_statement: NonNull<ffi::sqlite3_stmt>,
    /// The transaction control this statement performs when it completes.
    control: Option<ControlStatement>,
}

// This relies on the invariant that RawConnection or Statement are never
// leaked. If a reference to one of those was held on a different thread, this
// would not be thread safe.
#[allow(unsafe_code)]
unsafe impl Send for Statement {}

impl PreparedStatement for Statement {
    fn parameter_count(&self) -> u64 {
        // SAFETY: inner_statement is a valid sqlite3_stmt pointer to a non-deallocated statement
        unsafe {
            ffi::sqlite3_bind_parameter_count(self.inner_statement.as_ptr())
                .try_into()
                .unwrap_or(0)
        }
    }
}

impl Statement {
    pub(super) fn prepare(
        raw_connection: &RawConnection,
        sql: &str,
        is_cached: PrepareForCache,
        _: &[SqliteType],
    ) -> QueryResult<Self> {
        // the cast for `ffi::SQLITE_PREPARE_PERSISTENT` is required for old libsqlite3-sys versions
        #[allow(clippy::unnecessary_cast)]
        let flags = if matches!(is_cached, PrepareForCache::Yes { counter: _ }) {
            ffi::SQLITE_PREPARE_PERSISTENT as u32
        } else {
            0
        };
        let sql = CString::new(sql)?;
        let (statement, _) = Self::prepare_first(raw_connection, &sql, 0, flags)?;
        // sqlite3_prepare_v3 returns a null pointer for empty statements. This includes
        // empty or only whitespace strings or any other non-op query string like a comment
        statement.ok_or_else(|| {
            crate::result::Error::QueryBuilderError(Box::new(crate::result::EmptyQuery))
        })
    }

    /// Prepares the first statement of `sql[start..]`, returning `None` for text without one, and the offset where SQLite stopped.
    pub(super) fn prepare_first(
        raw_connection: &RawConnection,
        sql: &CStr,
        start: usize,
        flags: u32,
    ) -> QueryResult<(Option<Self>, usize)> {
        // A suffix of a C string's bytes is NUL terminated and holds no other NUL.
        let rest = &sql.to_bytes_with_nul()[start..];
        let mut stmt = ptr::null_mut();
        let mut tail = ptr::null();
        let capture = raw_connection.capture_control();
        // SAFETY: the connection is open, `rest` is NUL terminated and outlives the call, and
        // both output slots are writable.
        let prepare_result = unsafe {
            ffi::sqlite3_prepare_v3(
                raw_connection.internal_connection.as_ptr(),
                rest.as_ptr().cast(),
                -1,
                flags,
                &mut stmt,
                &mut tail,
            )
        };
        let control = capture.and_then(|capture| capture.finish());
        ensure_sqlite_ok(prepare_result, raw_connection.internal_connection.as_ptr())?;
        let statement = Self::from_prepared(stmt, control);
        let end = tail
            .addr()
            .checked_sub(sql.as_ptr().addr())
            .filter(|end| (start..=sql.count_bytes()).contains(end))
            .expect("SQLite returns a tail pointer into the prepared text");
        Ok((statement, end))
    }

    /// Steps the statement until it is done, discarding every row, then finalizes it.
    pub(super) fn run_to_completion(self) -> QueryResult<()> {
        loop {
            // SAFETY: `inner_statement` stays a live prepared statement until it is finalized.
            match unsafe { ffi::sqlite3_step(self.inner_statement.as_ptr()) } {
                ffi::SQLITE_ROW => {}
                ffi::SQLITE_DONE => break,
                // `Drop` finalizes, which only repeats this error.
                _ => return Err(last_error(self.raw_connection())),
            }
        }
        self.finalize()
    }

    /// Steps the statement until it is done, discarding every row and
    /// recording its transaction control, then finalizes it.
    pub(super) fn track_to_completion(
        mut self,
        raw_connection: &RawConnection,
        transaction_manager: &mut AnsiTransactionManager,
    ) -> QueryResult<()> {
        // `Drop` finalizes after an error, which only repeats it.
        while self.step(raw_connection, transaction_manager)? {}
        self.finalize()
    }

    fn finalize(mut self) -> QueryResult<()> {
        drop(self.control.take());
        let statement = core::mem::ManuallyDrop::new(self);
        let raw_connection = statement.raw_connection();
        // SAFETY: `ManuallyDrop` keeps `Drop` from finalizing the statement a second time.
        let result = unsafe { ffi::sqlite3_finalize(statement.inner_statement.as_ptr()) };
        ensure_sqlite_ok(result, raw_connection)
    }

    fn from_prepared(
        stmt: *mut ffi::sqlite3_stmt,
        control: Option<ControlStatement>,
    ) -> Option<Self> {
        let inner_statement = NonNull::new(stmt)?;
        // SAFETY: `inner_statement` was just prepared and is not finalized.
        let explains = unsafe { ffi::sqlite3_stmt_isexplain(inner_statement.as_ptr()) } != 0;
        Some(Statement {
            inner_statement,
            // `EXPLAIN` compiles the control without running it.
            control: control.filter(|_| !explains),
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
        value: SqliteBindValueRef<'_>,
        bind_index: i32,
    ) -> QueryResult<Option<NonNull<[u8]>>> {
        let mut ret_ptr = None;
        let result = match (tpe, value) {
            (_, SqliteBindValueRef::Null) => unsafe {
                ffi::sqlite3_bind_null(self.inner_statement.as_ptr(), bind_index)
            },
            (SqliteType::Binary, SqliteBindValueRef::BorrowedBinary(bytes)) => {
                let n = bytes
                    .len()
                    .try_into()
                    .map_err(|e| Error::SerializationError(Box::new(e)))?;
                unsafe {
                    ffi::sqlite3_bind_blob(
                        self.inner_statement.as_ptr(),
                        bind_index,
                        bytes.as_ptr() as *const libc::c_void,
                        n,
                        ffi::SQLITE_STATIC(),
                    )
                }
            }
            (SqliteType::Binary, SqliteBindValueRef::Binary(mut bytes)) => {
                let len = bytes
                    .len()
                    .try_into()
                    .map_err(|e| Error::SerializationError(Box::new(e)))?;
                // We need a separate pointer here to pass it to sqlite
                // as the returned pointer is a pointer to a dyn sized **slice**
                // and not the pointer to the first element of the slice
                let ptr = bytes.as_mut_ptr();
                ret_ptr = NonNull::new(Box::into_raw(bytes));
                unsafe {
                    ffi::sqlite3_bind_blob(
                        self.inner_statement.as_ptr(),
                        bind_index,
                        ptr as *const libc::c_void,
                        len,
                        ffi::SQLITE_STATIC(),
                    )
                }
            }
            (SqliteType::Text, SqliteBindValueRef::BorrowedString(bytes)) => {
                let len = bytes
                    .len()
                    .try_into()
                    .map_err(|e| Error::SerializationError(Box::new(e)))?;
                unsafe {
                    ffi::sqlite3_bind_text(
                        self.inner_statement.as_ptr(),
                        bind_index,
                        bytes.as_ptr() as *const libc::c_char,
                        len,
                        ffi::SQLITE_STATIC(),
                    )
                }
            }
            (SqliteType::Text, SqliteBindValueRef::String(bytes)) => {
                let mut bytes = Box::<[u8]>::from(bytes);
                let len = bytes
                    .len()
                    .try_into()
                    .map_err(|e| Error::SerializationError(Box::new(e)))?;
                // We need a separate pointer here to pass it to sqlite
                // as the returned pointer is a pointer to a dyn sized **slice**
                // and not the pointer to the first element of the slice
                let ptr = bytes.as_mut_ptr();
                ret_ptr = NonNull::new(Box::into_raw(bytes));
                unsafe {
                    ffi::sqlite3_bind_text(
                        self.inner_statement.as_ptr(),
                        bind_index,
                        ptr as *const libc::c_char,
                        len,
                        ffi::SQLITE_STATIC(),
                    )
                }
            }
            (SqliteType::Float, SqliteBindValueRef::F64(value))
            | (SqliteType::Double, SqliteBindValueRef::F64(value)) => unsafe {
                ffi::sqlite3_bind_double(
                    self.inner_statement.as_ptr(),
                    bind_index,
                    value as libc::c_double,
                )
            },
            (SqliteType::SmallInt, SqliteBindValueRef::I32(value))
            | (SqliteType::Integer, SqliteBindValueRef::I32(value)) => unsafe {
                ffi::sqlite3_bind_int(self.inner_statement.as_ptr(), bind_index, value)
            },
            (SqliteType::Long, SqliteBindValueRef::I64(value)) => unsafe {
                ffi::sqlite3_bind_int64(self.inner_statement.as_ptr(), bind_index, value)
            },
            (t, b) => {
                return Err(Error::SerializationError(
                    alloc::format!("Type mismatch: Expected {t:?}, got {b}").into(),
                ));
            }
        };
        match ensure_sqlite_ok(result, self.raw_connection()) {
            Ok(()) => Ok(ret_ptr),
            Err(e) => {
                if let Some(ptr) = ret_ptr {
                    // This is a `NonNul` ptr so it cannot be null
                    // It points to a slice internally as we did not apply
                    // any cast above.
                    core::mem::drop(unsafe { Box::from_raw(ptr.as_ptr()) })
                }
                Err(e)
            }
        }
    }

    fn reset(&mut self) {
        unsafe { ffi::sqlite3_reset(self.inner_statement.as_ptr()) };
    }

    /// Steps the statement once, recording a transaction control it completes
    /// and a transaction SQLite rolls back on its own.
    ///
    /// Returns whether a row is available.
    pub(super) fn step(
        &mut self,
        raw_connection: &RawConnection,
        transaction_manager: &mut AnsiTransactionManager,
    ) -> QueryResult<bool> {
        let inner_statement = self.inner_statement.as_ptr();
        // SQLite recompiles an expired statement inside `sqlite3_step`, consulting the authorizer again.
        let recompile = self.control.is_some().then(|| {
            (
                raw_connection.capture_control(),
                reprepare_count(inner_statement),
            )
        });
        // SAFETY: `inner_statement` stays a live prepared statement until `Drop`.
        let step_result = unsafe { ffi::sqlite3_step(inner_statement) };
        if let Some((capture, reprepares)) = recompile {
            let recompiled = capture.and_then(|capture| capture.finish());
            if reprepare_count(inner_statement) != reprepares
                && let (Some(control), Some(recompiled)) = (&mut self.control, recompiled)
            {
                control.allowed = recompiled.allowed;
            }
        }
        let db = self.raw_connection();
        // SAFETY: the statement keeps its connection open, so `db` is valid.
        let autocommit = || unsafe { ffi::sqlite3_get_autocommit(db) } != 0;
        match step_result {
            ffi::SQLITE_DONE => {
                if let Some(control) = self.control.as_ref().filter(|control| control.allowed) {
                    transaction_manager.complete_control(&control.control, autocommit());
                }
                Ok(false)
            }
            ffi::SQLITE_ROW => Ok(true),
            _ => {
                let error = last_error(db);
                // SQLite ends the transaction on some failures, e.g. `ON CONFLICT ROLLBACK`.
                if autocommit() {
                    transaction_manager.set_rolled_back_by_database();
                }
                Err(error)
            }
        }
    }

    fn clear_bindings(&mut self) {
        unsafe { ffi::sqlite3_clear_bindings(self.inner_statement.as_ptr()) };
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
    let error_code = last_error_code(raw_connection);
    let error_kind = match error_code {
        ffi::SQLITE_CONSTRAINT_UNIQUE | ffi::SQLITE_CONSTRAINT_PRIMARYKEY => {
            DatabaseErrorKind::UniqueViolation
        }
        ffi::SQLITE_CONSTRAINT_FOREIGNKEY => DatabaseErrorKind::ForeignKeyViolation,
        // SQLITE_CONSTRAINT_TRIGGER is returned for ON DELETE RESTRICT violations,
        // which are actually foreign key violations. We check the error message
        // to distinguish from user-defined trigger failures.
        ffi::SQLITE_CONSTRAINT_TRIGGER
            if error_message.contains("FOREIGN KEY constraint failed") =>
        {
            DatabaseErrorKind::ForeignKeyViolation
        }
        ffi::SQLITE_CONSTRAINT_NOTNULL => DatabaseErrorKind::NotNullViolation,
        ffi::SQLITE_CONSTRAINT_CHECK => DatabaseErrorKind::CheckViolation,
        _ => DatabaseErrorKind::Unknown,
    };
    let error_information = Box::new(SqliteErrorInformation {
        message: error_message,
        native_code: error_code,
    });
    DatabaseError(error_kind, error_information)
}

fn last_error_message(conn: *mut ffi::sqlite3) -> String {
    let c_str = unsafe { CStr::from_ptr(ffi::sqlite3_errmsg(conn)) };
    c_str.to_string_lossy().into_owned()
}

fn last_error_code(conn: *mut ffi::sqlite3) -> libc::c_int {
    unsafe { ffi::sqlite3_extended_errcode(conn) }
}

/// How often SQLite has recompiled `statement` since it was prepared.
fn reprepare_count(statement: *mut ffi::sqlite3_stmt) -> libc::c_int {
    // SAFETY: callers pass a live prepared statement, and reading a counter without resetting it has no other effect.
    unsafe { ffi::sqlite3_stmt_status(statement, ffi::SQLITE_STMTSTATUS_REPREPARE, 0) }
}

impl Drop for Statement {
    fn drop(&mut self) {
        // An error only repeats the last failed step, which already returned it.
        unsafe { ffi::sqlite3_finalize(self.inner_statement.as_ptr()) };
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
        mut statement: MaybeCached<'stmt, Statement>,
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
        // make sure to reset any previously left bindings
        statement.reset();
        statement.clear_bindings();

        let mut bind_collector = SqliteBindCollector::new();
        query.collect_binds(&mut bind_collector, &mut (), &Sqlite)?;

        let SqliteBindCollector { binds } = bind_collector;
        if statement.parameter_count() != binds.len() as u64 {
            return Err(
                crate::connection::statement_cache::invalid_prepared_statement_parameter_count(
                    statement.parameter_count().try_into().unwrap_or(usize::MAX),
                    binds.len(),
                ),
            );
        }

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
        binds: Vec<(SqliteBindValueRef<'_>, SqliteType)>,
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
                        SqliteBindValueRef::BorrowedBinary(_)
                            | SqliteBindValueRef::BorrowedString(_)
                            | SqliteBindValueRef::String(_)
                            | SqliteBindValueRef::Binary(_)
                    )
                })
                .count(),
        );
        for (bind_idx, (bind, tpe)) in (1..).zip(binds) {
            let is_borrowed_bind = matches!(
                bind,
                SqliteBindValueRef::BorrowedString(_) | SqliteBindValueRef::BorrowedBinary(_)
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

impl Drop for BoundStatement<'_, '_> {
    fn drop(&mut self) {
        // First reset the statement, otherwise the bind calls
        // below will fails
        self.statement.reset();

        for (idx, buffer) in core::mem::take(&mut self.binds_to_free) {
            unsafe {
                // It's always safe to bind null values, as there is no buffer that needs to outlife something
                self.statement
                    .bind(SqliteType::Text, SqliteBindValueRef::Null, idx)
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
                    core::mem::drop(Box::from_raw(buffer.as_ptr()));
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
            core::mem::drop(query);
            self.query = None;
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct StatementUse<'stmt, 'query> {
    statement: BoundStatement<'stmt, 'query>,
    // SAFETY:
    // https://sqlite.org/c3ref/column_name.html
    // This pointer is valid as long as:
    // The returned string pointer is valid until either
    // the prepared statement is destroyed by sqlite3_finalize()
    // or until the statement is automatically reprepared by the first
    // call to sqlite3_step() for a particular run or until the next call
    // to sqlite3_column_name() or sqlite3_column_name16() on the same column.
    //
    // Step has an explicit `first` flag that resets this field
    // in this case
    // The `OnceCell` ensure that we never call
    // `sqlite3_column_name` again after populating this field
    //
    // We also assume that we are the owner of `statement` and the
    // inner pointer is not exposed anywhere else. This ensures
    // no one else can call the functions mentioned above
    //
    // We use `*const str` instead of `&str`
    // as this is essentially a self referential struct
    // as we borrow into `statement`
    column_names: OnceCell<Vec<*const str>>,
    transaction_manager: &'stmt mut AnsiTransactionManager,
    raw_connection: &'stmt RawConnection,
}

impl<'stmt, 'query> StatementUse<'stmt, 'query> {
    pub(super) fn bind<T>(
        statement: MaybeCached<'stmt, Statement>,
        query: T,
        instrumentation: &'stmt mut dyn Instrumentation,
        transaction_manager: &'stmt mut AnsiTransactionManager,
        raw_connection: &'stmt RawConnection,
    ) -> QueryResult<StatementUse<'stmt, 'query>>
    where
        T: QueryFragment<Sqlite> + QueryId + 'query,
    {
        Ok(Self {
            statement: BoundStatement::bind(statement, query, instrumentation)?,
            column_names: OnceCell::new(),
            transaction_manager,
            raw_connection,
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
        let res = self
            .statement
            .statement
            .step(self.raw_connection, self.transaction_manager);
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
            let column_name = unsafe {
                ffi::sqlite3_column_name(self.statement.statement.inner_statement.as_ptr(), idx)
            };
            assert!(
                !column_name.is_null(),
                "The Sqlite documentation states that it only returns a \
                 null pointer here if we are in a OOM condition."
            );
            unsafe { CStr::from_ptr(column_name) }
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
            .map(|v| {
                v.try_into()
                    .expect("Diesel expects to run at least on a 32 bit platform")
            })
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
            .get(usize::try_from(idx).expect("Diesel expects to run at least on a 32 bit platform"))
            .and_then(|c| unsafe { c.as_ref() })
    }

    pub(super) fn copy_value(&self, idx: i32) -> QueryResult<Option<OwnedSqliteValue>> {
        match self.column_value(idx) {
            Some(ptr) => OwnedSqliteValue::copy_from_ptr(ptr),
            None => Ok(None),
        }
    }

    pub(super) fn column_value(&self, idx: i32) -> Option<NonNull<ffi::sqlite3_value>> {
        let ptr = unsafe {
            ffi::sqlite3_column_value(self.statement.statement.inner_statement.as_ptr(), idx)
        };
        NonNull::new(ptr)
    }

    pub(super) fn raw_connection(&self) -> *mut ffi::sqlite3 {
        self.statement.statement.raw_connection()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(miri))]
    use crate::prelude::*;
    #[cfg(not(miri))]
    use crate::sql_types::Text;

    // this is a regression test for
    // https://github.com/diesel-rs/diesel/issues/3558
    //
    // Reading sqlite's error message requires accessing memory allocated by
    // the native library, which is not supported when running under miri with
    // a native libsqlite3 (`-Zmiri-native-lib`).
    #[cfg(not(miri))] // ffi call
    #[diesel_test_helper::test]
    fn check_out_of_bounds_bind_does_not_panic_on_drop() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();

        let e = crate::sql_query("SELECT '?'")
            .bind::<Text, _>("foo")
            .execute(&mut conn);

        assert!(e.is_err());
        let e = e.unwrap_err();
        assert_eq!(
            e.to_string(),
            "prepared statement expected a different number of parameters than provided: Expected 0, provided: 1"
        );
    }
}
