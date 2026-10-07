//! Sqlite jsonb, reached by binding a parameter and reading the column back.

use diesel::deserialize::FromSqlRow;
use diesel::expression::TypedExpressionType;
use diesel::prelude::*;
use diesel::query_builder::QueryId;
use diesel::sql_types::{
    BigInt, Binary, Bool, Date, Double, Float, Integer, Json, Jsonb, Numeric, SingleValue,
    SmallInt, SqlType, Text, Time, Timestamp, TimestamptzSqlite,
};
use diesel::sqlite::JsonValidFlag;
use diesel::sqlite::Sqlite;
use diesel::sqlite::expression::dsl::json_valid_with_flags;
use std::cell::RefCell;

thread_local! {
    static CONN: RefCell<SqliteConnection> = RefCell::new(
        SqliteConnection::establish(":memory:").expect("an in-memory sqlite database")
    );
}

pub fn with_conn<R>(f: impl FnOnce(&mut SqliteConnection) -> R) -> R {
    CONN.with(|conn| f(&mut conn.borrow_mut()))
}

pub fn decode_jsonb(conn: &mut SqliteConnection, blob: &[u8]) -> QueryResult<serde_json::Value> {
    diesel::select(diesel::dsl::sql::<Jsonb>("").bind::<Binary, _>(blob)).get_result(conn)
}

/// Writes `value` as jsonb, then requires diesel to read it back unchanged and
/// sqlite to call those very bytes well formed. Sqlite's own rendering is not
/// compared because `json()` prints a double to fifteen significant digits,
/// which does not round trip, so a comparison reports sqlite's loss as a defect.
pub fn roundtrip_jsonb(
    conn: &mut SqliteConnection,
    value: &serde_json::Value,
) -> Result<(), String> {
    let blob: Vec<u8> =
        diesel::select(diesel::dsl::sql::<Binary>("").bind::<Jsonb, _>(value.clone()))
            .get_result(conn)
            .map_err(|e| format!("diesel cannot write {value}: {e}"))?;

    let back = decode_jsonb(conn, &blob)
        .map_err(|e| format!("diesel cannot read back the blob it wrote for {value}: {e}"))?;
    if &back != value {
        return Err(format!("{value} read back as {back}"));
    }

    match jsonb_valid(conn, &blob) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "sqlite calls the blob diesel wrote for {value} malformed: {blob:02X?}"
        )),
        Err(e) => Err(format!("sqlite cannot judge the blob for {value}: {e}")),
    }
}

/// Writes `value` as json text, then requires diesel to read it back unchanged
/// and sqlite to call the text well formed.
pub fn roundtrip_json(
    conn: &mut SqliteConnection,
    value: &serde_json::Value,
) -> Result<(), String> {
    let text: String = diesel::select(diesel::dsl::sql::<Text>("").bind::<Json, _>(value.clone()))
        .get_result(conn)
        .map_err(|e| format!("diesel cannot write {value} as json text: {e}"))?;

    let back: serde_json::Value =
        diesel::select(diesel::dsl::sql::<Json>("").bind::<Text, _>(text.clone()))
            .get_result(conn)
            .map_err(|e| format!("diesel cannot read back the json text {text:?}: {e}"))?;
    if &back != value {
        return Err(format!("{value} read back as {back} through json text"));
    }

    let valid: bool = diesel::select(json_valid_with_flags::<Text, _, _>(
        text.as_str(),
        JsonValidFlag::Rfc8259Json,
    ))
    .get_result(conn)
    .map_err(|e| format!("sqlite cannot judge the json text {text:?}: {e}"))?;
    if !valid {
        return Err(format!("sqlite calls the json text {text:?} malformed"));
    }
    Ok(())
}

/// Checks conformance to sqlite's own jsonb format. A query error is no verdict
/// on the blob, so it stays an error rather than becoming one.
pub fn jsonb_valid(conn: &mut SqliteConnection, blob: &[u8]) -> QueryResult<bool> {
    diesel::select(json_valid_with_flags::<Binary, _, _>(
        blob,
        JsonValidFlag::JsonbStrict,
    ))
    .get_result(conn)
}

/// Binds `bytes` in one of the four storage classes sqlite has, then reads the
/// column back as `T`, so a decoder sees a value sqlite itself produced.
fn probe<ST, T>(conn: &mut SqliteConnection, kind: u8, bytes: &[u8]) -> QueryResult<T>
where
    ST: SqlType + TypedExpressionType + SingleValue + QueryId,
    Sqlite: diesel::sql_types::HasSqlType<ST>,
    T: FromSqlRow<ST, Sqlite> + 'static,
{
    let column = diesel::dsl::sql::<ST>("");
    match kind % 4 {
        0 => diesel::select(column.bind::<Binary, _>(bytes.to_vec())).get_result(conn),
        1 => diesel::select(column.bind::<Text, _>(String::from_utf8_lossy(bytes).into_owned()))
            .get_result(conn),
        2 => diesel::select(column.bind::<BigInt, _>(as_i64(bytes))).get_result(conn),
        _ => {
            diesel::select(column.bind::<Double, _>(f64::from_bits(as_u64(bytes)))).get_result(conn)
        }
    }
}

fn as_u64(bytes: &[u8]) -> u64 {
    let mut buffer = [0u8; 8];
    let taken = bytes.len().min(8);
    buffer[..taken].copy_from_slice(&bytes[..taken]);
    u64::from_le_bytes(buffer)
}

fn as_i64(bytes: &[u8]) -> i64 {
    i64::from_le_bytes(as_u64(bytes).to_le_bytes())
}

macro_rules! cases {
    ( $( ($name:literal, $ST:ty, $T:ty) ),* $(,)? ) => {
        pub const CASES: &[&str] = &[ $( $name ),* ];

        pub fn decode_case(selector: u8, kind: u8, bytes: &[u8]) {
            with_conn(|conn| match CASES[usize::from(selector) % CASES.len()] {
                $( $name => { let _ = probe::<$ST, $T>(conn, kind, bytes); } )*
                _ => unreachable!(),
            });
        }
    };
}

cases!(
    ("bool", Bool, bool),
    ("i16", SmallInt, i16),
    ("i32", Integer, i32),
    ("i64", BigInt, i64),
    ("f32", Float, f32),
    ("f64", Double, f64),
    ("text", Text, String),
    ("binary", Binary, Vec<u8>),
    ("bigdecimal", Numeric, bigdecimal::BigDecimal),
    ("date_text", Date, String),
    ("time_text", Time, String),
    ("timestamp_text", Timestamp, String),
    ("timestamptz_text", TimestamptzSqlite, String),
    ("chrono_date", Date, chrono::NaiveDate),
    ("chrono_time", Time, chrono::NaiveTime),
    ("chrono_datetime", Timestamp, chrono::NaiveDateTime),
    ("chrono_naive_tz", TimestamptzSqlite, chrono::NaiveDateTime),
    ("chrono_utc", TimestamptzSqlite, chrono::DateTime<chrono::Utc>),
    ("chrono_local", TimestamptzSqlite, chrono::DateTime<chrono::Local>),
    ("time_date", Date, time::Date),
    ("time_time", Time, time::Time),
    ("time_primitive", Timestamp, time::PrimitiveDateTime),
    ("time_primitive_tz", TimestamptzSqlite, time::PrimitiveDateTime),
    ("time_offset", TimestamptzSqlite, time::OffsetDateTime),
    ("json", Json, serde_json::Value),
    ("jsonb", Jsonb, serde_json::Value),
);
