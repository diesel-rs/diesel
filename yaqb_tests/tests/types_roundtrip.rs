use quickcheck::quickcheck;

use schema::connection;
use yaqb::*;
use yaqb::result::Error;
use yaqb::types::structs::*;
use yaqb::types::{NativeSqlType, ToSql, Nullable, Array};

fn test_type_round_trips<ST, T>(value: T, type_name: &str) -> bool where
    ST: NativeSqlType,
    T: ToSql<ST> + Queriable<ST> + PartialEq,
{
    let connection = connection();
    let query = format!("SELECT $1::{}", type_name);
    let result = connection.query_sql_params::<ST, T, ST, T>(&query, &value);
    match result {
        Ok(mut val) => value == val.nth(0).unwrap(),
        Err(Error::DatabaseError(msg)) =>
            &msg == "ERROR:  invalid byte sequence for encoding \"UTF8\": 0x00\n",
        Err(e) => panic!("Query failed: {:?}", e),
    }
}

macro_rules! test_round_trip {
    ($test_name:ident, $sql_type:ident, $tpe:ty, $sql_type_name:expr) => {
        #[test]
        fn $test_name() {
            fn round_trip(val: $tpe) -> bool {
                test_type_round_trips::<types::$sql_type, _>(val, $sql_type_name)
            }

            fn option_round_trip(val: Option<$tpe>) -> bool {
                test_type_round_trips::<Nullable<types::$sql_type>, _>(val, $sql_type_name)
            }

            fn vec_round_trip(val: Vec<$tpe>) -> bool {
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

macro_rules! test_newtype_round_trip {
    ($test_name:ident, $sql_type:ident, $newtype:ident, $tpe:ty, $sql_type_name:expr) => {
        #[test]
        fn $test_name() {
            fn round_trip(val: $tpe) -> bool {
                test_type_round_trips::<types::$sql_type, _>($newtype(val), $sql_type_name)
            }

            fn option_round_trip(val: Option<$tpe>) -> bool {
                let val = val.map($newtype);
                test_type_round_trips::<Nullable<types::$sql_type>, _>(val, $sql_type_name)
            }

            fn vec_round_trip(val: Vec<$tpe>) -> bool {
                let val: Vec<_> = val.into_iter().map($newtype).collect();
                test_type_round_trips::<Array<types::$sql_type>, _>(val, concat!($sql_type_name, "[]"))
            }

            quickcheck(round_trip as fn($tpe) -> bool);
            quickcheck(option_round_trip as fn(Option<$tpe>) -> bool);
            quickcheck(vec_round_trip as fn(Vec<$tpe>) -> bool);
        }
    }
}

test_newtype_round_trip!(date_roundtrips, Date, PgDate, i32, "date");
test_newtype_round_trip!(time_roundtrips, Time, to_pg_time, i64, "time");
test_newtype_round_trip!(timestamp_roundtrips, Timestamp, PgTimestamp, i64, "timestamp");
test_newtype_round_trip!(interval_roundtrips, Interval, to_pg_interval, (i64, i32, i32), "interval");

fn to_pg_time(int: i64) -> PgTime {
    PgTime(::std::cmp::max(0, int))
}

fn to_pg_interval(vals: (i64, i32, i32)) -> PgInterval {
    let (microseconds, days, months) = vals;
    PgInterval {
        microseconds: microseconds,
        days: days,
        months: months,
    }
}
