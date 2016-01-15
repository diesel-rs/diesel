extern crate chrono;

pub use quickcheck::quickcheck;
use self::chrono::NaiveDateTime;

pub use schema::connection;
pub use diesel::*;
pub use diesel::result::Error;
pub use diesel::data_types::*;
pub use diesel::types::{NativeSqlType, ToSql, Nullable, Array};

pub fn test_type_round_trips<ST, T>(value: T, type_name: &str) -> bool where
    ST: NativeSqlType,
    T: ToSql<ST> + Queryable<ST> + PartialEq + ::std::fmt::Debug,
{
    let connection = connection();
    let query = format!("SELECT $1::{}", type_name);
    let result = connection.query_sql_params::<ST, T, ST, T>(&query, &value);
    match result {
        Ok(mut val) => {
            let res = val.nth(0).unwrap();
            if value != res {
                println!("{:?}, {:?}", value, res);
                false
            } else {
                true
            }
        }
        Err(Error::DatabaseError(msg)) =>
            &msg == "ERROR:  invalid byte sequence for encoding \"UTF8\": 0x00\n",
        Err(e) => panic!("Query failed: {:?}", e),
    }
}

pub fn id<A>(a: A) -> A { a }

macro_rules! test_round_trip {
    ($test_name:ident, $sql_type:ident, $tpe:ty, $sql_type_name:expr) => {
        test_round_trip!($test_name, $sql_type, $tpe, id, $sql_type_name);
    };

    ($test_name:ident, $sql_type:ident, $tpe:ty, $map_fn:ident, $sql_type_name:expr) => {
        #[test]
        fn $test_name() {
            fn round_trip(val: $tpe) -> bool {
                test_type_round_trips::<types::$sql_type, _>($map_fn(val), $sql_type_name)
            }

            fn option_round_trip(val: Option<$tpe>) -> bool {
                let val = val.map($map_fn);
                test_type_round_trips::<Nullable<types::$sql_type>, _>(val, $sql_type_name)
            }

            fn vec_round_trip(val: Vec<$tpe>) -> bool {
                let val: Vec<_> = val.into_iter().map($map_fn).collect();
                test_type_round_trips::<Array<types::$sql_type>, _>(val, concat!($sql_type_name, "[]"))
            }

            quickcheck(round_trip as fn($tpe) -> bool);
            quickcheck(option_round_trip as fn(Option<$tpe>) -> bool);
            quickcheck(vec_round_trip as fn(Vec<$tpe>) -> bool);
        }
    }
}

test_round_trip!(bool_roundtrips, Bool, bool, "boolean");
test_round_trip!(i16_roundtrips, SmallInt, i16, "int2");
test_round_trip!(i32_roundtrips, Integer, i32, "int4");
test_round_trip!(i64_roundtrips, BigInt, i64, "int8");
test_round_trip!(f32_roundtrips, Float, f32, "real");
test_round_trip!(f64_roundtrips, Double, f64, "double precision");
test_round_trip!(string_roundtrips, VarChar, String, "varchar");
test_round_trip!(text_roundtrips, Text, String, "text");
test_round_trip!(binary_roundtrips, Binary, Vec<u8>, "bytea");
test_round_trip!(date_roundtrips, Date, PgDate, "date");
test_round_trip!(time_roundtrips, Time, PgTime, "time");
test_round_trip!(timestamp_roundtrips, Timestamp, PgTimestamp, "timestamp");
test_round_trip!(interval_roundtrips, Interval, PgInterval, "interval");
test_round_trip!(numeric_roundtrips, Numeric, PgNumeric, "numeric");
test_round_trip!(naive_datetime_roundtrips, Timestamp, (i64, u32), mk_naive_datetime, "timestamp");

fn mk_naive_datetime(data: (i64, u32)) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(data.0, data.1 / 1000)
}

#[cfg(feature = "unstable")]
mod unstable_types {
    use super::*;
    use std::time::*;

    fn strip_nanosecond_precision(time: SystemTime) -> SystemTime {
        let res = match time.duration_from_earlier(UNIX_EPOCH) {
            Ok(duration) => time - Duration::new(0, duration.subsec_nanos() % 1000),
            Err(e) => time + Duration::new(0, e.duration().subsec_nanos() % 1000),
        };
        work_around_rust_lang_30173(res)
    }

    fn work_around_rust_lang_30173(time: SystemTime) -> SystemTime {
        time + Duration::new(0, 1) - Duration::new(0, 1)
    }

    test_round_trip!(systemtime_roundtrips, Timestamp, SystemTime, strip_nanosecond_precision, "timestamp");
}
