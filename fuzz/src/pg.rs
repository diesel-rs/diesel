//! Every postgres `FromSql` impl that decodes bytes, so a panic is a finding.

use std::num::NonZeroU32;
use std::ops::Bound;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use diesel::deserialize::FromSql;
use diesel::pg::data_types::{
    NdArray, PgDate, PgInterval, PgLsn as PgLsnValue, PgMoney, PgNumeric, PgTime, PgTimestamp,
};
use diesel::pg::{Pg, PgValue};
use diesel::sql_types::{
    Array, BigInt, Binary, Bool, CChar, Cidr, Citext, Date, Double, Float, Inet, Integer, Interval,
    Json, Jsonb, MacAddr, MacAddr8, Money, Multirange, Nullable, Numeric, Oid, PgLsn, Range,
    Record, SmallInt, Text, Time, Timestamp, Timestamptz, Uuid,
};
use ipnet::IpNet;
use ipnetwork::IpNetwork;
use time::{OffsetDateTime, PrimitiveDateTime};

fn decode<ST, T>(bytes: &[u8], oid: NonZeroU32) -> diesel::deserialize::Result<T>
where
    T: FromSql<ST, Pg>,
{
    T::from_sql(PgValue::new(bytes, &oid))
}

macro_rules! cases {
    ( $( ($name:literal, $ST:ty, $T:ty) ),* $(,)? ) => {
        pub const CASES: &[&str] = &[ $( $name ),* ];

        pub fn decode_case(selector: u8, oid: NonZeroU32, bytes: &[u8]) {
            match CASES[usize::from(selector) % CASES.len()] {
                $( $name => { let _ = decode::<$ST, $T>(bytes, oid); } )*
                _ => unreachable!(),
            }
        }
    };
}

cases!(
    ("bool", Bool, bool),
    ("i16", SmallInt, i16),
    ("i32", Integer, i32),
    ("i64", BigInt, i64),
    ("u32", Oid, u32),
    ("f32", Float, f32),
    ("f64", Double, f64),
    ("text", Text, String),
    ("cchar", CChar, u8),
    ("citext", Citext, String),
    ("binary", Binary, Vec<u8>),
    ("pg_numeric", Numeric, PgNumeric),
    ("bigdecimal", Numeric, BigDecimal),
    ("pg_money", Money, PgMoney),
    ("pg_timestamp", Timestamp, PgTimestamp),
    ("chrono_naive_dt", Timestamp, NaiveDateTime),
    ("time_primitive_dt", Timestamp, PrimitiveDateTime),
    ("chrono_naive_dt_tz", Timestamptz, NaiveDateTime),
    ("chrono_dt_utc", Timestamptz, DateTime<Utc>),
    ("chrono_dt_local", Timestamptz, DateTime<Local>),
    ("pg_timestamp_tz", Timestamptz, PgTimestamp),
    ("system_time", Timestamp, std::time::SystemTime),
    ("time_prim_dt_tz", Timestamptz, PrimitiveDateTime),
    ("time_offset_dt", Timestamptz, OffsetDateTime),
    ("pg_date", Date, PgDate),
    ("chrono_date", Date, NaiveDate),
    ("time_date", Date, time::Date),
    ("pg_time", Time, PgTime),
    ("chrono_time", Time, NaiveTime),
    ("time_time", Time, time::Time),
    ("pg_interval", Interval, PgInterval),
    ("chrono_interval", Interval, chrono::Duration),
    ("uuid", Uuid, uuid::Uuid),
    ("inet_ipnet", Inet, IpNet),
    ("inet_ipnetwork", Inet, IpNetwork),
    ("cidr_ipnet", Cidr, IpNet),
    ("cidr_ipnetwork", Cidr, IpNetwork),
    ("mac_addr", MacAddr, [u8; 6]),
    ("mac_addr8", MacAddr8, [u8; 8]),
    ("pg_lsn", PgLsn, PgLsnValue),
    ("json", Json, serde_json::Value),
    ("jsonb", Jsonb, serde_json::Value),
    ("array_i32", Array<Integer>, Vec<i32>),
    ("array_opt_text", Array<Nullable<Text>>, Vec<Option<String>>),
    ("ndarray_i32", Array<Integer>, NdArray<i32>),
    ("range_i32", Range<Integer>, (Bound<i32>, Bound<i32>)),
    ("multirange_i32", Multirange<Integer>, Vec<(Bound<i32>, Bound<i32>)>),
    ("record", Record<(Integer, Text)>, (i32, String)),
);
