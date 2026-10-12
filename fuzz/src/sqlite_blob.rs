//! SQLite incremental blob operations checked against a byte-slice model.

use arbitrary::Arbitrary;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sql_types::Binary;
use std::cell::RefCell;
use std::io::{ErrorKind, Read, Seek, SeekFrom};

/// Large enough for any `SqliteBlobOp::Read` length.
const READ_BUFFER_SIZE: usize = 256;

/// One operation on the blob handle.
#[derive(Arbitrary, Clone, Copy, Debug)]
pub enum SqliteBlobOp {
    /// Read into a buffer of this many bytes.
    Read(u8),
    /// Seek to this offset from the start.
    SeekStart(u16),
    /// Seek to this offset from the end.
    SeekEnd(i16),
    /// Seek by this offset from the cursor.
    SeekCurrent(i16),
}

#[derive(Arbitrary, Debug)]
pub struct BlobInput<'a> {
    /// Bytes stored in the SQLite blob.
    pub data: &'a [u8],
    /// Operations applied in order.
    pub operations: Vec<SqliteBlobOp>,
    /// Whether to explicitly close the handle instead of dropping it.
    pub close_explicitly: bool,
}

diesel::table! {
    fuzz_blobs (id) {
        id -> Integer,
        data -> Binary,
    }
}

thread_local! {
    static CONN: RefCell<SqliteConnection> = RefCell::new(new_connection());
}

fn new_connection() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").expect("an in-memory sqlite database");
    conn.batch_execute(
        "CREATE TABLE fuzz_blobs (id INTEGER PRIMARY KEY, data BLOB NOT NULL); \
         INSERT INTO fuzz_blobs (id, data) VALUES (1, X'');",
    )
    .expect("the blob fuzz fixture to initialize");
    conn
}

/// Run the input's blob operations, then close or drop the handle.
pub fn run_case(input: &BlobInput<'_>) -> Result<(), String> {
    CONN.with(|conn| {
        run_on_connection(
            &mut conn.borrow_mut(),
            input.data,
            &input.operations,
            input.close_explicitly,
        )
    })
}

fn run_on_connection(
    conn: &mut SqliteConnection,
    data: &[u8],
    operations: &[SqliteBlobOp],
    close_explicitly: bool,
) -> Result<(), String> {
    let updated = diesel::sql_query("UPDATE fuzz_blobs SET data = ? WHERE id = 1")
        .bind::<Binary, _>(data)
        .execute(conn)
        .map_err(|e| format!("failed to update blob fixture: {e}"))?;
    if updated != 1 {
        return Err(format!(
            "updated {updated} blob fixture rows instead of one"
        ));
    }

    let mut blob = conn.get_read_only_blob(fuzz_blobs::data, 1).map_err(|e| {
        format!(
            "failed to open {blob_len}-byte blob: {e}",
            blob_len = data.len()
        )
    })?;
    if blob.len() != data.len() {
        return Err(format!(
            "opened blob has length {}, expected {}",
            blob.len(),
            data.len()
        ));
    }

    let mut cursor = 0;
    for (operation_index, &operation) in operations.iter().enumerate() {
        let (seek_from, expected_cursor) = match operation {
            SqliteBlobOp::Read(requested) => {
                let requested = usize::from(requested);
                let expected_len = (data.len() - cursor).min(requested);
                let mut output = [0; READ_BUFFER_SIZE];
                let actual_len = blob
                    .read(&mut output[..requested])
                    .map_err(|e| format!("read operation {operation_index} failed: {e}"))?;
                if actual_len != expected_len {
                    return Err(format!(
                        "read operation {operation_index} returned {actual_len} bytes, expected {expected_len}"
                    ));
                }
                let expected = &data[cursor..cursor + expected_len];
                if &output[..actual_len] != expected {
                    return Err(format!(
                        "read operation {operation_index} returned {:?}, expected {expected:?}",
                        &output[..actual_len]
                    ));
                }
                cursor += actual_len;
                continue;
            }
            SqliteBlobOp::SeekStart(offset) => (
                SeekFrom::Start(u64::from(offset)),
                Some(usize::from(offset).min(data.len())),
            ),
            SqliteBlobOp::SeekEnd(offset) => {
                let expected = if offset.is_positive() {
                    Some(data.len())
                } else {
                    data.len().checked_sub(usize::from(offset.unsigned_abs()))
                };
                (SeekFrom::End(i64::from(offset)), expected)
            }
            SqliteBlobOp::SeekCurrent(offset) => {
                let magnitude = usize::from(offset.unsigned_abs());
                let expected = if offset.is_negative() {
                    cursor.checked_sub(magnitude)
                } else {
                    Some(cursor.saturating_add(magnitude).min(data.len()))
                };
                (SeekFrom::Current(i64::from(offset)), expected)
            }
        };
        let actual_cursor = match (blob.seek(seek_from), expected_cursor) {
            (Ok(position), Some(_)) => position,
            (Err(error), None) if error.kind() == ErrorKind::InvalidInput => {
                blob.stream_position().map_err(|e| {
                    format!("position after rejected seek {operation_index} failed: {e}")
                })?
            }
            (actual, expected) => {
                return Err(format!(
                    "seek operation {operation_index} returned {actual:?}, expected {expected:?}"
                ));
            }
        };
        let expected_cursor = expected_cursor.unwrap_or(cursor);
        let expected_cursor_u64 =
            u64::try_from(expected_cursor).expect("the modeled blob length fits in u64");
        if actual_cursor != expected_cursor_u64 {
            return Err(format!(
                "seek operation {operation_index} returned {actual_cursor}, expected {expected_cursor}"
            ));
        }
        cursor = expected_cursor;
    }

    if close_explicitly {
        blob.close()
            .map_err(|e| format!("failed to close blob explicitly: {e}"))?;
    }
    Ok(())
}
