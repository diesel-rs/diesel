//! Every mysql `FromSql` impl, under every wire type the server can report.
//! These decoders are generic over `MysqlLikeBackend`, so this covers MariaDB too.

use bigdecimal::BigDecimal;
use diesel::deserialize::FromSql;
use diesel::mysql::data_types::MysqlTime;
use diesel::mysql::{Mysql, MysqlType, MysqlValue};
use diesel::sql_types::{
    BigInt, Binary, Bool, Date, Datetime, Double, Float, Integer, Json, Numeric, SmallInt, Text,
    Time, Timestamp, TinyInt, Unsigned,
};

fn decode<ST, T>(bytes: &[u8], tpe: MysqlType) -> diesel::deserialize::Result<T>
where
    T: FromSql<ST, Mysql>,
{
    T::from_sql(MysqlValue::new(bytes, tpe))
}

macro_rules! cases {
    ( $( ($name:literal, $ST:ty, $T:ty) ),* $(,)? ) => {
        pub const CASES: &[&str] = &[ $( $name ),* ];

        pub fn decode_case(selector: u8, type_selector: u8, bytes: &[u8]) {
            let tpe = TYPES[usize::from(type_selector) % TYPES.len()];
            match CASES[usize::from(selector) % CASES.len()] {
                $( $name => { let _ = decode::<$ST, $T>(bytes, tpe); } )*
                _ => unreachable!(),
            }
        }
    };
}

cases!(
    ("i8", TinyInt, i8),
    ("u8", Unsigned<TinyInt>, u8),
    ("i16", SmallInt, i16),
    ("u16", Unsigned<SmallInt>, u16),
    ("i32", Integer, i32),
    ("u32", Unsigned<Integer>, u32),
    ("i64", BigInt, i64),
    ("u64", Unsigned<BigInt>, u64),
    ("f32", Float, f32),
    ("f64", Double, f64),
    ("bigdecimal", Numeric, BigDecimal),
    ("bool", Bool, bool),
    ("text", Text, String),
    ("binary", Binary, Vec<u8>),
    ("mysql_time_date", Date, MysqlTime),
    ("mysql_time_time", Time, MysqlTime),
    ("mysql_time_datetime", Datetime, MysqlTime),
    ("mysql_time_timestamp", Timestamp, MysqlTime),
    ("chrono_date", Date, chrono::NaiveDate),
    ("chrono_time", Time, chrono::NaiveTime),
    ("chrono_dt_ts", Timestamp, chrono::NaiveDateTime),
    ("chrono_dt", Datetime, chrono::NaiveDateTime),
    ("time_date", Date, time::Date),
    ("time_time", Time, time::Time),
    ("time_primitive_dt", Datetime, time::PrimitiveDateTime),
    ("time_primitive_ts", Timestamp, time::PrimitiveDateTime),
    ("time_offset_dt", Datetime, time::OffsetDateTime),
    ("time_offset_ts", Timestamp, time::OffsetDateTime),
    ("json", Json, serde_json::Value),
);

pub const TYPES: &[MysqlType] = &[
    MysqlType::Tiny,
    MysqlType::UnsignedTiny,
    MysqlType::Short,
    MysqlType::UnsignedShort,
    MysqlType::Long,
    MysqlType::UnsignedLong,
    MysqlType::LongLong,
    MysqlType::UnsignedLongLong,
    MysqlType::Float,
    MysqlType::Double,
    MysqlType::Numeric,
    MysqlType::Time,
    MysqlType::Date,
    MysqlType::DateTime,
    MysqlType::Timestamp,
    MysqlType::String,
    MysqlType::Blob,
    MysqlType::Bit,
    MysqlType::Set,
    MysqlType::Enum,
];
