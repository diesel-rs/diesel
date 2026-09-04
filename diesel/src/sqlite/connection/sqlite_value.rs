#![allow(unsafe_code)] // ffi calls
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use std::cell::Ref;
use std::ptr::NonNull;
use std::{slice, str};

use crate::sqlite::SqliteType;
use crate::QueryResult;

use super::owned_row::OwnedSqliteRow;
use super::row::PrivateSqliteRow;

/// Raw sqlite value as received from the database
///
/// Use the `read_*` functions to access the actual
/// value or use existing `FromSql` implementations
/// to convert this into rust values
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct SqliteValue<'row, 'stmt, 'query> {
    // This field exists to ensure that nobody can modify the underlying row
    // while we are holding a reference to some row value here, and to reach
    // the connection a value is still attached to
    owner: ValueOwner<'row, 'stmt, 'query>,
    // we extract the raw value pointer as part of the constructor
    // to safe the match statements for each method
    // According to benchmarks this leads to a ~20-30% speedup
    //
    // This is sound as long as nobody calls `stmt.step()`
    // while holding this value. We ensure this by including
    // a reference to the row above.
    value: NonNull<ffi::sqlite3_value>,
    // An optional storage for a string that is
    // created from an non-utf8 blob value via `read_str`
    // This field mostly exists for as we cannot
    // return an error in that case as the API is
    // stable and doesn't return a `Result`. We instead
    // use `String::from_utf8_lossy` there and need
    // to store the potential owned result here
    string_ref: Option<Box<str>>,
    // The type the value declared before any read converted it, as
    // https://www.sqlite.org/c3ref/value_blob.html requires asking first
    initial_type: SqliteType,
    // A private copy of the value, used by reads that would otherwise convert the
    // shared value in place and free the buffer another handle points at
    converted: Option<OwnedSqliteValue>,
}

enum ValueOwner<'row, 'stmt, 'query> {
    // Only a direct row is still attached to its connection, a duplicated one
    // holds values from `sqlite3_value_dup`
    Row(Ref<'row, PrivateSqliteRow<'stmt, 'query>>),
    // A value outside a row: a function argument carries its connection, a
    // duplicated value has none
    NonRow(Option<NonNull<ffi::sqlite3>>),
}

/// A form a value read hands out, which SQLite stores by converting the value.
#[derive(Copy, Clone)]
enum Representation {
    Text,
    Blob,
}

impl Representation {
    fn target(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Blob => "a blob",
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub(super) struct OwnedSqliteValue {
    pub(super) value: NonNull<ffi::sqlite3_value>,
}

impl Drop for OwnedSqliteValue {
    fn drop(&mut self) {
        unsafe { ffi::sqlite3_value_free(self.value.as_ptr()) }
    }
}

// Unsafe Send impl safe since sqlite3_value is built with sqlite3_value_dup
// see https://www.sqlite.org/c3ref/value.html
unsafe impl Send for OwnedSqliteValue {}

#[cold]
fn allocation_failed(out_of_memory: bool, target: &str) -> ! {
    let reason = if out_of_memory {
        "ran out of memory"
    } else {
        "failed to allocate memory"
    };
    panic!("SQLite {reason} while reading a value as {target}")
}

#[cold]
fn duplication_failed() -> crate::result::Error {
    crate::result::Error::DeserializationError(
        "SQLite failed to allocate a duplicated value".into(),
    )
}

impl<'row, 'stmt, 'query> SqliteValue<'row, 'stmt, 'query> {
    pub(super) fn new(
        row: Ref<'row, PrivateSqliteRow<'stmt, 'query>>,
        col_idx: usize,
    ) -> Option<SqliteValue<'row, 'stmt, 'query>> {
        let value = match &*row {
            PrivateSqliteRow::Direct(stmt) => stmt.column_value(
                col_idx
                    .try_into()
                    .expect("Diesel expects to run at least on a 32 bit platform"),
            )?,
            PrivateSqliteRow::Duplicated { values, .. } => {
                values.get(col_idx).and_then(|v| v.as_ref())?.value
            }
        };
        // SAFETY: the row owns the value and keeps it alive for `'row`.
        let initial_type = unsafe { value_type_of(value) }?;
        Some(Self {
            owner: ValueOwner::Row(row),
            value,
            string_ref: None,
            initial_type,
            converted: None,
        })
    }

    pub(super) fn from_owned_row(
        row: &'row OwnedSqliteRow,
        col_idx: usize,
    ) -> Option<SqliteValue<'row, 'stmt, 'query>> {
        let value = row.values.get(col_idx).and_then(|v| v.as_ref())?.value;
        // SAFETY: `row` owns the value and keeps it alive for `'row`.
        let initial_type = unsafe { value_type_of(value) }?;
        Some(Self {
            owner: ValueOwner::NonRow(None),
            value,
            string_ref: None,
            initial_type,
            converted: None,
        })
    }

    pub(super) fn from_function_row(
        row: &'row [Option<OwnedSqliteValue>],
        col_idx: usize,
        connection: NonNull<ffi::sqlite3>,
    ) -> Option<SqliteValue<'row, 'stmt, 'query>> {
        let value = row.get(col_idx).and_then(|v| v.as_ref())?.value;
        // SAFETY: the callback's argument row owns the value for the call.
        let initial_type = unsafe { value_type_of(value) }?;
        Some(Self {
            owner: ValueOwner::NonRow(Some(connection)),
            value,
            string_ref: None,
            initial_type,
            converted: None,
        })
    }

    // Values from `sqlite3_value_dup` are disconnected from the connection, so they
    // cannot be asked: https://www.sqlite.org/c3ref/value_blob.html
    fn connection(&self) -> Option<NonNull<ffi::sqlite3>> {
        match &self.owner {
            ValueOwner::Row(row) => match &**row {
                PrivateSqliteRow::Direct(stmt) => NonNull::new(stmt.raw_connection()),
                PrivateSqliteRow::Duplicated { .. } => None,
            },
            ValueOwner::NonRow(connection) => *connection,
        }
    }

    /// Reports whether an allocation just failed, which must be asked before any other call.
    fn reports_out_of_memory(&self) -> bool {
        let Some(connection) = self.connection() else {
            return false;
        };
        // SAFETY: The owner keeps the connection alive and this call only reads
        // connection state.
        unsafe { ffi::sqlite3_errcode(connection.as_ptr()) == ffi::SQLITE_NOMEM }
    }

    /// Returns the value to read `wanted` from, copying it first when SQLite would
    /// convert the shared value in place and free a buffer another handle points at.
    fn value_to_read(&mut self, wanted: Representation) -> NonNull<ffi::sqlite3_value> {
        // A value gains its other textual form by conversion, while a numeric one
        // gains it in a fresh buffer that replaces nothing.
        let converts_in_place = matches!(
            (self.initial_type, wanted),
            (SqliteType::Text, Representation::Blob) | (SqliteType::Binary, Representation::Text)
        );
        if !converts_in_place {
            return self.value;
        }
        if self.converted.is_none() {
            // SAFETY: `self.owner` keeps `self.value` alive across this call, which
            // only reads it while copying it into an independently owned value.
            let copy = unsafe { ffi::sqlite3_value_dup(self.value.as_ptr()) };
            // The value above is not SQL NULL, so only a failed allocation is null.
            let Some(copy) = NonNull::new(copy) else {
                // Ask the connection before any other call to it clears the error code.
                allocation_failed(self.reports_out_of_memory(), wanted.target());
            };
            self.converted = Some(OwnedSqliteValue { value: copy });
        }
        self.converted
            .as_ref()
            .expect("We initialised it literally above")
            .value
    }

    pub(crate) fn as_byte_string(&mut self) -> &[u8] {
        let value = self.value_to_read(Representation::Text);
        // SAFETY: `self.owner` keeps the value alive, a copy is owned by `self`, and
        // the returned slice borrows `self` for as long as either can be converted.
        unsafe {
            // Force the UTF-8 conversion now so the length below cannot
            // trigger one (moving the buffer, or failing as zero length).
            // https://www.sqlite.org/c3ref/column_blob.html
            if ffi::sqlite3_value_text(value.as_ptr()).is_null() {
                // Zero length text has a valid pointer, so this is a failed conversion.
                allocation_failed(self.reports_out_of_memory(), Representation::Text.target());
            }
            let len = ffi::sqlite3_value_bytes(value.as_ptr());
            // The length above may have invalidated the pointer.
            let ptr = ffi::sqlite3_value_text(value.as_ptr());
            if ptr.is_null() {
                allocation_failed(self.reports_out_of_memory(), Representation::Text.target());
            }
            slice::from_raw_parts(
                ptr,
                len.try_into()
                    .expect("Diesel expects to run at least on a 32 bit platform"),
            )
        }
    }

    pub(crate) fn as_utf8_str(&mut self) -> Result<&str, core::str::Utf8Error> {
        str::from_utf8(self.as_byte_string())
    }

    pub(crate) fn parse_string<'value, R>(&'value mut self, f: impl FnOnce(&'value str) -> R) -> R {
        // For blobs this might return non-utf values
        //
        // The sqlite documentation there seems to be at least inaccurate
        if str::from_utf8(self.as_byte_string()).is_err() {
            // Read again to drop the byte borrow before storing the lossy copy, as
            // repeated reads of one value do not convert it again.
            let lossy = String::from_utf8_lossy(self.as_byte_string()).into_owned();
            self.string_ref = Some(lossy.into_boxed_str());
            let s = self
                .string_ref
                .as_deref()
                .expect("We initialised it literally above");
            return f(s);
        }
        let s = str::from_utf8(self.as_byte_string()).expect("The bytes are valid utf8 above");
        f(s)
    }

    /// Read the underlying value as string
    ///
    /// If the underlying value is not a string sqlite will convert it
    /// into a string and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    ///
    /// Reading a blob value as text copies it, so slices returned for the same
    /// field elsewhere stay valid.
    ///
    /// # Panics
    ///
    /// Panics if SQLite cannot allocate the requested text representation.
    pub fn read_text(&mut self) -> &str {
        // TODO: Return Result in Diesel 3 so SQLite allocation failures reach callers.
        self.parse_string(|s| s)
    }

    /// Read the underlying value as blob
    ///
    /// If the underlying value is not a blob sqlite will convert it
    /// into a blob and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    ///
    /// Zero-length text and blob values are read as an empty slice even when
    /// SQLite reports a null pointer for them.
    ///
    /// Reading a text value as a blob copies it, so slices returned for the same
    /// field elsewhere stay valid.
    ///
    /// # Panics
    ///
    /// Panics if SQLite cannot allocate the requested blob representation.
    pub fn read_blob(&mut self) -> &[u8] {
        // TODO: Return Result in Diesel 3 so SQLite allocation failures reach callers.
        let value = self.value_to_read(Representation::Blob);
        // SAFETY: as in `as_byte_string`, the owner keeps the value alive, `self` owns
        // a copy, and the returned slice borrows `self` for as long as either lives.
        unsafe {
            // Preserve a zeroblob's length because failed expansion changes it to SQL NULL.
            let initial_blob_len = if matches!(self.initial_type, SqliteType::Binary) {
                Some(ffi::sqlite3_value_bytes(value.as_ptr()))
            } else {
                None
            };
            // Pin the blob form before the length: bytes() must not measure
            // (or fail at) a text conversion of the value instead.
            // https://www.sqlite.org/c3ref/column_blob.html
            if ffi::sqlite3_value_blob(value.as_ptr()).is_null() {
                // Ask the connection before any other call to it clears the error code.
                let out_of_memory = self.reports_out_of_memory();
                let len = ffi::sqlite3_value_bytes(value.as_ptr());
                if !out_of_memory
                    && ((matches!(self.initial_type, SqliteType::Text) && len == 0)
                        || initial_blob_len == Some(0))
                {
                    return &[];
                }
                allocation_failed(out_of_memory, Representation::Blob.target());
            }
            let len = ffi::sqlite3_value_bytes(value.as_ptr());
            // The length above may have invalidated the pointer.
            let ptr = ffi::sqlite3_value_blob(value.as_ptr());
            if ptr.is_null() {
                allocation_failed(self.reports_out_of_memory(), Representation::Blob.target());
            }
            slice::from_raw_parts(
                ptr as *const u8,
                len.try_into()
                    .expect("Diesel expects to run at least on a 32 bit platform"),
            )
        }
    }

    /// Read the underlying value as 32 bit integer
    ///
    /// If the underlying value is not an integer sqlite will convert it
    /// into an integer and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_integer(&mut self) -> i32 {
        unsafe { ffi::sqlite3_value_int(self.value.as_ptr()) }
    }

    /// Read the underlying value as 64 bit integer
    ///
    /// If the underlying value is not a string sqlite will convert it
    /// into a string and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_long(&mut self) -> i64 {
        unsafe { ffi::sqlite3_value_int64(self.value.as_ptr()) }
    }

    /// Read the underlying value as 64 bit float
    ///
    /// If the underlying value is not a string sqlite will convert it
    /// into a string and return that value instead.
    ///
    /// Use the [`value_type()`](Self::value_type()) function to determine the actual
    /// type of the value.
    ///
    /// See <https://www.sqlite.org/c3ref/value_blob.html> for details
    pub fn read_double(&mut self) -> f64 {
        unsafe { ffi::sqlite3_value_double(self.value.as_ptr()) }
    }

    /// Get the type of the value as returned by sqlite
    pub fn value_type(&self) -> Option<SqliteType> {
        // SAFETY: `self.owner` keeps the value alive and this call only inspects it.
        unsafe { value_type_of(self.value) }
    }
}

/// Reads a value's type, which SQLite reports as SQL `NULL` for a failed conversion.
///
/// # Safety
///
/// `value` must point to a live `sqlite3_value`.
unsafe fn value_type_of(value: NonNull<ffi::sqlite3_value>) -> Option<SqliteType> {
    // SAFETY: the caller guarantees a live value, which this call only inspects.
    let tpe = unsafe { ffi::sqlite3_value_type(value.as_ptr()) };
    match tpe {
        ffi::SQLITE_TEXT => Some(SqliteType::Text),
        ffi::SQLITE_INTEGER => Some(SqliteType::Long),
        ffi::SQLITE_FLOAT => Some(SqliteType::Double),
        ffi::SQLITE_BLOB => Some(SqliteType::Binary),
        ffi::SQLITE_NULL => None,
        _ => unreachable!(
            "Sqlite's documentation state that this case ({}) is not reachable. \
             If you ever see this error message please open an issue at \
             https://github.com/diesel-rs/diesel.",
            tpe
        ),
    }
}

impl OwnedSqliteValue {
    /// Copies a value out of a statement or a function argument.
    ///
    /// `Ok(None)` is SQL `NULL`. A failed allocation is an error instead, as reporting
    /// it as `NULL` would hand out a wrong value.
    pub(super) fn copy_from_ptr(
        ptr: NonNull<ffi::sqlite3_value>,
    ) -> QueryResult<Option<OwnedSqliteValue>> {
        // SAFETY: `ptr` points to a live `sqlite3_value` owned by the statement or
        // callback that outlives this call, and reading its type only inspects it.
        let tpe = unsafe { ffi::sqlite3_value_type(ptr.as_ptr()) };
        if ffi::SQLITE_NULL == tpe {
            return Ok(None);
        }
        // SAFETY: the same live value as above, which `sqlite3_value_dup` only reads
        // while it copies it into an independently owned value.
        let value = unsafe { ffi::sqlite3_value_dup(ptr.as_ptr()) };
        // The value above is not null, so only a failed allocation returns null here.
        let value = NonNull::new(value).ok_or_else(duplication_failed)?;
        Ok(Some(Self { value }))
    }

    pub(super) fn duplicate(&self) -> QueryResult<OwnedSqliteValue> {
        // SAFETY: `self` owns `self.value` and keeps it alive across this call, and
        // `sqlite3_value_dup` only reads it while copying it.
        let value = unsafe { ffi::sqlite3_value_dup(self.value.as_ptr()) };
        let value = NonNull::new(value).ok_or_else(duplication_failed)?;
        Ok(OwnedSqliteValue { value })
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::{LoadConnection, SimpleConnection};
    use crate::row::Field;
    use crate::row::Row;
    use crate::sql_types::{Blob, Double, Int4, Text};
    use crate::*;

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    mod allocation_failure {
        use super::super::SqliteValue;
        use crate::connection::{LoadConnection, SimpleConnection};
        use crate::deserialize::{self, FromSql};
        use crate::prelude::*;
        use crate::row::{Field, Row};
        use crate::sql_types::Binary;
        use crate::sqlite::connection::oom_test_support::{
            panic_message, run_in_child, with_heap_limit,
        };
        use crate::sqlite::{Sqlite, SqliteConnection};

        const VALUE_LEN: usize = 1_048_576;

        crate::table! {
            oom_blob (id) {
                id -> Integer,
                value -> Binary,
            }
        }

        crate::table! {
            oom_text (id) {
                id -> Integer,
                value -> Text,
            }
        }

        crate::table! {
            oom_zeroblob (id) {
                id -> Integer,
                len -> Integer,
            }
        }

        crate::define_sql_function! {
            fn read_blob_under_pressure(value: Binary) -> Integer;
        }

        /// Carries the panic message of a blob read that ran out of memory, as the
        /// failing statement reports SQLite's own error instead.
        struct BlobUnderPressure(String);

        impl FromSql<Binary, Sqlite> for BlobUnderPressure {
            fn from_sql(mut value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
                let payload = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
                    without_spare_memory(|| core::hint::black_box(value.read_blob().len()));
                }))
                .expect_err("the blob read did not panic");
                Ok(Self(panic_message(&*payload).to_string()))
            }
        }

        impl crate::deserialize::Queryable<Binary, Sqlite> for BlobUnderPressure {
            type Row = Self;

            fn build(row: Self::Row) -> deserialize::Result<Self> {
                Ok(row)
            }
        }

        /// Rejects every further SQLite allocation while `f` runs.
        fn without_spare_memory<R>(f: impl FnOnce() -> R) -> R {
            with_heap_limit(0, f)
        }

        fn expect_panic(f: impl FnOnce() + core::panic::UnwindSafe, expected: &str) {
            let payload = std::panic::catch_unwind(f).expect_err("the read did not panic");
            let message = panic_message(&*payload);
            assert!(
                message.contains(expected),
                "unexpected panic message: {message}"
            );
        }

        fn blob_connection(rows: i32) -> SqliteConnection {
            let mut conn = SqliteConnection::establish(":memory:").unwrap();
            // Diesel has no typed DDL.
            conn.batch_execute(
                "CREATE TABLE oom_blob (id INTEGER PRIMARY KEY, value BLOB NOT NULL)",
            )
            .unwrap();
            for id in 1..=rows {
                crate::insert_into(oom_blob::table)
                    .values((
                        oom_blob::id.eq(id),
                        oom_blob::value.eq(vec![b'x'; VALUE_LEN]),
                    ))
                    .execute(&mut conn)
                    .unwrap();
            }
            conn
        }

        fn utf16_text_connection(rows: i32) -> SqliteConnection {
            let mut conn = SqliteConnection::establish(":memory:").unwrap();
            // Diesel has no typed DDL and no typed representation of the database encoding.
            conn.batch_execute(
                "PRAGMA encoding = 'UTF-16le';
                 CREATE TABLE oom_text (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
            )
            .unwrap();
            for id in 1..=rows {
                crate::insert_into(oom_text::table)
                    .values((
                        oom_text::id.eq(id),
                        oom_text::value.eq("x".repeat(VALUE_LEN)),
                    ))
                    .execute(&mut conn)
                    .unwrap();
            }
            conn
        }

        #[test]
        fn text_read_panics_when_conversion_fails() {
            run_in_child(|| {
                let mut conn = utf16_text_connection(1);
                let mut rows = conn.load(oom_text::table.select(oom_text::value)).unwrap();
                let row = rows.next().unwrap().unwrap();
                let field = row.get(0).unwrap();
                let mut value = field.value().unwrap();

                expect_panic(
                    core::panic::AssertUnwindSafe(|| {
                        without_spare_memory(|| {
                            core::hint::black_box(value.read_text().len());
                        });
                    }),
                    "SQLite ran out of memory while reading a value as text",
                );
            });
        }

        #[test]
        fn duplicated_text_read_panics_when_conversion_fails() {
            run_in_child(|| {
                let mut conn = utf16_text_connection(2);
                let mut rows = conn.load(oom_text::table.select(oom_text::value)).unwrap();
                let first = rows.next().unwrap().unwrap();
                // Advancing while the first row lives copies its values out of the statement,
                // and `sqlite3_value_dup` disconnects them from the connection.
                let _second = rows.next().unwrap().unwrap();
                let field = first.get(0).unwrap();
                let mut value = field.value().unwrap();

                expect_panic(
                    core::panic::AssertUnwindSafe(|| {
                        without_spare_memory(|| {
                            core::hint::black_box(value.read_text().len());
                        });
                    }),
                    "SQLite failed to allocate memory while reading a value as text",
                );
            });
        }

        #[test]
        fn blob_read_as_text_panics_when_the_copy_fails() {
            run_in_child(|| {
                let mut conn = blob_connection(1);
                let mut rows = conn.load(oom_blob::table.select(oom_blob::value)).unwrap();
                let row = rows.next().unwrap().unwrap();
                let field = row.get(0).unwrap();
                let mut value = field.value().unwrap();

                expect_panic(
                    core::panic::AssertUnwindSafe(|| {
                        without_spare_memory(|| {
                            core::hint::black_box(value.read_text().len());
                        });
                    }),
                    "SQLite failed to allocate memory while reading a value as text",
                );
            });
        }

        #[test]
        fn text_read_as_blob_panics_when_the_copy_fails() {
            run_in_child(|| {
                let mut conn = utf16_text_connection(1);
                let mut rows = conn.load(oom_text::table.select(oom_text::value)).unwrap();
                let row = rows.next().unwrap().unwrap();
                let field = row.get(0).unwrap();
                let mut value = field.value().unwrap();

                expect_panic(
                    core::panic::AssertUnwindSafe(|| {
                        without_spare_memory(|| {
                            core::hint::black_box(value.read_blob().len());
                        });
                    }),
                    "SQLite failed to allocate memory while reading a value as a blob",
                );
            });
        }

        #[test]
        fn blob_read_keeps_utf16_text_bytes_across_a_text_read() {
            let mut conn = utf16_text_connection(1);
            let mut rows = conn.load(oom_text::table.select(oom_text::value)).unwrap();
            let row = rows.next().unwrap().unwrap();
            let field = row.get(0).unwrap();
            let mut blob_value = field.value().unwrap();
            let mut text_value = field.value().unwrap();

            let blob = blob_value.read_blob();
            assert_eq!(blob.len(), 2 * VALUE_LEN);
            let (pairs, rest) = blob.as_chunks::<2>();
            assert!(
                pairs.iter().all(|pair| pair == b"x\0") && rest.is_empty(),
                "SQLite blob content changed"
            );

            // Converting the shared value to UTF-8 frees its UTF-16 buffer.
            assert_eq!(text_value.read_text().len(), VALUE_LEN);
            let (pairs, rest) = blob.as_chunks::<2>();
            assert!(
                pairs.iter().all(|pair| pair == b"x\0") && rest.is_empty(),
                "the text read invalidated the blob bytes"
            );
        }

        #[test]
        fn zeroblob_read_panics_when_expansion_fails() {
            run_in_child(|| {
                let mut conn = SqliteConnection::establish(":memory:").unwrap();
                // Diesel has no typed DDL.
                conn.batch_execute(
                    "CREATE TABLE oom_zeroblob (id INTEGER PRIMARY KEY, len INTEGER NOT NULL)",
                )
                .unwrap();
                crate::insert_into(oom_zeroblob::table)
                    .values((
                        oom_zeroblob::id.eq(1),
                        oom_zeroblob::len.eq(i32::try_from(VALUE_LEN).unwrap()),
                    ))
                    .execute(&mut conn)
                    .unwrap();
                let observed = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
                let callback_observed = std::sync::Arc::clone(&observed);
                read_blob_under_pressure_utils::register_impl(
                    &mut conn,
                    move |value: BlobUnderPressure| {
                        assert!(
                            value.0.contains(
                                "SQLite ran out of memory while reading a value as a blob"
                            ),
                            "unexpected panic message: {}",
                            value.0
                        );
                        callback_observed.store(true, core::sync::atomic::Ordering::Relaxed);
                        1
                    },
                )
                .unwrap();

                // Diesel has no typed representation of `zeroblob`, and a constant argument
                // would be expanded by the virtual machine before the function sees it.
                let result = oom_zeroblob::table
                    .select(read_blob_under_pressure(crate::dsl::sql::<Binary>(
                        "zeroblob(len)",
                    )))
                    .get_result::<i32>(&mut conn);
                assert!(result.is_err(), "the blob read did not fail the statement");
                assert!(
                    observed.load(core::sync::atomic::Ordering::Relaxed),
                    "the function did not read its argument"
                );
            });
        }

        #[test]
        fn row_duplication_reports_value_duplication_failure() {
            run_in_child(|| {
                let mut conn = blob_connection(2);
                let mut rows = conn.load(oom_blob::table.select(oom_blob::value)).unwrap();
                // Holding the first row makes the next step copy it out of the statement.
                let _first = rows.next().unwrap().unwrap();

                let error = match without_spare_memory(|| rows.next()) {
                    Some(Err(e)) => e,
                    Some(Ok(_)) => panic!("the row duplication did not fail"),
                    None => panic!("the iterator ended instead of copying the row"),
                };
                assert!(
                    error
                        .to_string()
                        .contains("SQLite failed to allocate a duplicated value"),
                    "unexpected error: {error}"
                );
            });
        }
    }

    crate::table! {
        empty_values (id) {
            id -> Integer,
            text -> Text,
            blob -> Binary,
            zero_blob -> Binary,
        }
    }

    #[diesel_test_helper::test]
    fn can_read_empty_values_as_empty_blob() {
        use crate::prelude::*;
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        // Diesel has no typed DDL, and BLOB affinity keeps the empty text literal in
        // `blob` as TEXT, which the typed DSL cannot store there.
        conn.batch_execute(
            "CREATE TABLE empty_values (id INTEGER PRIMARY KEY, text TEXT, blob BLOB, zero_blob BLOB);
             INSERT INTO empty_values (id, text, blob, zero_blob) VALUES (1, '', '', X'');",
        )
        .unwrap();

        // The same empty TEXT through the typed `FromSql<Binary, Sqlite>` path.
        let loaded = crate::select(crate::dsl::sql::<crate::sql_types::Binary>("''"))
            .get_result::<Vec<u8>>(&mut conn)
            .unwrap();
        assert!(loaded.is_empty());

        let mut rows = conn
            .load(empty_values::table.select((
                empty_values::text,
                empty_values::blob,
                empty_values::zero_blob,
            )))
            .unwrap();
        let row = rows.next().unwrap().unwrap();
        let text_field = row.get(0).unwrap();
        let blob_field = row.get(1).unwrap();
        let zero_blob_field = row.get(2).unwrap();

        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_text(), "");
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_blob(), b"");

        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.value_type(), Some(super::SqliteType::Text));
        assert_eq!(blob_value.read_blob(), b"");

        // A zero length BLOB, for which SQLite also reports a null pointer.
        let mut zero_blob_value = zero_blob_field.value().unwrap();
        assert_eq!(
            zero_blob_value.value_type(),
            Some(super::SqliteType::Binary)
        );
        assert_eq!(zero_blob_value.read_blob(), b"");
    }

    #[diesel_test_helper::test]
    fn blob_bytes_survive_a_text_read_of_the_same_value() {
        use crate::prelude::*;
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        // Diesel has no typed `randomblob`, and a stored blob points into the page
        // image, which a conversion never frees.
        let mut rows = conn
            .load(crate::select(crate::dsl::sql::<crate::sql_types::Binary>(
                "randomblob(1048576)",
            )))
            .unwrap();
        let row = rows.next().unwrap().unwrap();
        let field = row.get(0).unwrap();
        let mut blob_value = field.value().unwrap();
        let mut text_value = field.value().unwrap();
        let mut other_blob_value = field.value().unwrap();

        let blob = blob_value.read_blob();
        let expected = Vec::from(blob);
        let address = blob.as_ptr();

        // Converting the shared value to text would free the buffer `blob` points at.
        assert!(!text_value.read_text().is_empty());
        assert_eq!(blob, expected.as_slice(), "the text read moved the blob");
        assert_eq!(
            other_blob_value.read_blob().as_ptr(),
            address,
            "the text read converted the shared value"
        );
    }

    #[expect(clippy::approx_constant)] // we really want to use 3.14
    #[diesel_test_helper::test]
    fn can_convert_all_values() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();

        conn.batch_execute("CREATE TABLE tests(int INTEGER, text TEXT, blob BLOB, float FLOAT)")
            .unwrap();

        diesel::sql_query("INSERT INTO tests(int, text, blob, float) VALUES(?, ?, ?, ?)")
            .bind::<Int4, _>(42)
            .bind::<Text, _>("foo")
            .bind::<Blob, _>([0xFF_u8, 0xFE, 0xFD])
            .bind::<Double, _>(3.14)
            .execute(&mut conn)
            .unwrap();

        let mut res = conn
            .load(diesel::sql_query(
                "SELECT int, text, blob, float FROM tests",
            ))
            .unwrap();
        let row = res.next().unwrap().unwrap();
        let int_field = row.get(0).unwrap();
        let text_field = row.get(1).unwrap();
        let blob_field = row.get(2).unwrap();
        let float_field = row.get(3).unwrap();

        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_integer(), 42);
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_long(), 42);
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_double(), 42.0);
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_text(), "42");
        let mut int_value = int_field.value().unwrap();
        assert_eq!(int_value.read_blob(), b"42");

        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_integer(), 0);
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_long(), 0);
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_double(), 0.0);
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_text(), "foo");
        let mut text_value = text_field.value().unwrap();
        assert_eq!(text_value.read_blob(), b"foo");

        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_integer(), 0);
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_long(), 0);
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_double(), 0.0);
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_text(), "\u{fffd}\u{fffd}\u{fffd}"); // ���
        let mut blob_value = blob_field.value().unwrap();
        assert_eq!(blob_value.read_blob(), [0xFF, 0xFE, 0xFD]);

        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_integer(), 3);
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_long(), 3);
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_double(), 3.14);
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_text(), "3.14");
        let mut float_value = float_field.value().unwrap();
        assert_eq!(float_value.read_blob(), b"3.14");
    }
}
