extern crate chrono;

pub use quickcheck::quickcheck;
use self::chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, DateTime, Utc};
use self::chrono::naive;

pub use schema::{connection, TestConnection};
pub use diesel::*;
pub use diesel::result::Error;
pub use diesel::data_types::*;
pub use diesel::types::{HasSqlType, ToSql};

use diesel::expression::AsExpression;
use diesel::query_builder::{QueryFragment, QueryId};

pub fn test_type_round_trips<ST, T>(value: T) -> bool where
    ST: QueryId,
    <TestConnection as Connection>::Backend: HasSqlType<ST>,
    T: AsExpression<ST> + Queryable<ST, <TestConnection as Connection>::Backend> + PartialEq + Clone + ::std::fmt::Debug,
    <T as AsExpression<ST>>::Expression: SelectableExpression<(), SqlType=ST> + QueryFragment<<TestConnection as Connection>::Backend> + QueryId,
{
    let connection = connection();
    let query = select(AsExpression::<ST>::as_expression(value.clone()));
    let result = query.get_result::<T>(&connection);
    match result {
        Ok(res) => {
            if value != res {
                println!("{:?}, {:?}", value, res);
                false
            } else {
                true
            }
        }
        Err(Error::DatabaseError(_, ref e))
            if e.message() == "invalid byte sequence for encoding \"UTF8\": 0x00" => true,
        Err(e) => panic!("Query failed: {:?}", e),
    }
}

pub fn id<A>(a: A) -> A { a }

macro_rules! test_round_trip {
    ($test_name:ident, $sql_type:ty, $tpe:ty) => {
        test_round_trip!($test_name, $sql_type, $tpe, id);
    };

    ($test_name:ident, $sql_type:ty, $tpe:ty, $map_fn:ident) => {
        #[test]
        fn $test_name() {
            use diesel::types::*;

            fn round_trip(val: $tpe) -> bool {
                test_type_round_trips::<$sql_type, _>($map_fn(val))
            }

            fn option_round_trip(val: Option<$tpe>) -> bool {
                let val = val.map($map_fn);
                test_type_round_trips::<Nullable<$sql_type>, _>(val)
            }

            quickcheck(round_trip as fn($tpe) -> bool);
            quickcheck(option_round_trip as fn(Option<$tpe>) -> bool);
        }
    }
}

test_round_trip!(i16_roundtrips, SmallInt, i16);
test_round_trip!(i32_roundtrips, Integer, i32);
test_round_trip!(i64_roundtrips, BigInt, i64);
test_round_trip!(f32_roundtrips, Float, f32);
test_round_trip!(f64_roundtrips, Double, f64);
test_round_trip!(string_roundtrips, VarChar, String);
test_round_trip!(text_roundtrips, Text, String);
test_round_trip!(binary_roundtrips, Binary, Vec<u8>);
test_round_trip!(bool_roundtrips, Bool, bool);

#[cfg(feature = "sqlite")]
mod sqlite_types {
    use super::*;

    test_round_trip!(date_roundtrips, Date, String);
    test_round_trip!(time_roundtrips, Time, String);
    test_round_trip!(timestamp_roundtrips, Timestamp, String);
    test_round_trip!(naive_time_roundtrips, Time, (u32, u32), mk_naive_time);
    test_round_trip!(naive_date_roundtrips, Date, u32, mk_naive_date);
    test_round_trip!(naive_datetime_roundtrips, Timestamp, (i64, u32), mk_naive_datetime);
}

#[cfg(feature = "postgres")]
mod pg_types {
    extern crate uuid;
    extern crate ipnetwork;
    use super::*;

    test_round_trip!(date_roundtrips, Date, PgDate);
    test_round_trip!(time_roundtrips, Time, PgTime);
    test_round_trip!(timestamp_roundtrips, Timestamp, PgTimestamp);
    test_round_trip!(interval_roundtrips, Interval, PgInterval);
    test_round_trip!(numeric_roundtrips, Numeric, PgNumeric);
    test_round_trip!(money_roundtrips, Money, PgMoney);
    test_round_trip!(naive_datetime_roundtrips, Timestamp, (i64, u32), mk_naive_datetime);
    test_round_trip!(naive_time_roundtrips, Time, (u32, u32), mk_naive_time);
    test_round_trip!(naive_date_roundtrips, Date, u32, mk_naive_date);
    test_round_trip!(datetime_roundtrips, Timestamptz, (i64, u32), mk_datetime);
    test_round_trip!(naive_datetime_roundtrips_tz, Timestamptz, (i64, u32), mk_naive_datetime);
    test_round_trip!(uuid_roundtrips, Uuid, (u32, u16, u16, (u8, u8, u8, u8, u8, u8, u8, u8)), mk_uuid);
    test_round_trip!(array_of_int_roundtrips, Array<Integer>, Vec<i32>);
    test_round_trip!(array_of_bigint_roundtrips, Array<BigInt>, Vec<i64>);
    test_round_trip!(array_of_dynamic_size_roundtrips, Array<Text>, Vec<String>);
    test_round_trip!(array_of_nullable_roundtrips, Array<Nullable<Text>>, Vec<Option<String>>);
    test_round_trip!(macaddr_roundtrips, MacAddr, (u8, u8, u8, u8, u8, u8), mk_macaddr);
    test_round_trip!(cidr_v4_roundtrips, Cidr, (u8, u8, u8, u8), mk_ipv4);
    test_round_trip!(cidr_v6_roundtrips, Cidr, (u16, u16, u16, u16, u16, u16, u16, u16), mk_ipv6);
    test_round_trip!(inet_v4_roundtrips, Inet, (u8, u8, u8, u8), mk_ipv4);
    test_round_trip!(inet_v6_roundtrips, Inet, (u16, u16, u16, u16, u16, u16, u16, u16), mk_ipv6);

    fn mk_uuid(data: (u32, u16, u16, (u8, u8, u8, u8, u8, u8, u8, u8))) -> self::uuid::Uuid {
        let a = data.3;
        let b = [a.0, a.1, a.2, a.3, a.4, a.5, a.6, a.7];
        uuid::Uuid::from_fields(data.0, data.1, data.2, &b).unwrap()
    }

    fn mk_macaddr(data: (u8, u8, u8, u8, u8, u8)) -> [u8; 6] {
        [data.0, data.1, data.2, data.3, data.4, data.5]
    }

    fn mk_ipv4(data: (u8,u8, u8, u8)) -> ipnetwork::IpNetwork {
        use std::net::Ipv4Addr;
        let ip = Ipv4Addr::new(data.0, data.1, data.2, data.3);
        ipnetwork::IpNetwork::V4(ipnetwork::Ipv4Network::new(ip, 32).unwrap())
    }

    fn mk_ipv6(data: (u16, u16, u16, u16, u16, u16, u16, u16)) -> ipnetwork::IpNetwork {
        use std::net::Ipv6Addr;
        let ip = Ipv6Addr::new(data.0, data.1, data.2, data.3, data.4, data.5, data.6, data.7);
        ipnetwork::IpNetwork::V6(ipnetwork::Ipv6Network::new(ip, 128).unwrap())
    }
}

#[cfg(feature = "mysql")]
mod mysql_types {
    use super::*;

    test_round_trip!(naive_datetime_roundtrips, Timestamp, (i64, u32), mk_naive_datetime);
    test_round_trip!(naive_time_roundtrips, Time, (u32, u32), mk_naive_time);
    test_round_trip!(naive_date_roundtrips, Date, u32, mk_naive_date);
}

pub fn mk_naive_datetime(data: (i64, u32)) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(data.0, data.1 / 1000)
}

pub fn mk_naive_time(data: (u32, u32)) -> NaiveTime {
    NaiveTime::from_num_seconds_from_midnight(data.0, data.1 / 1000)
}

pub fn mk_datetime(data: (i64, u32)) -> DateTime<Utc> {
    DateTime::from_utc(mk_naive_datetime(data), Utc)
}

#[cfg(feature = "postgres")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    let earliest_pg_date = NaiveDate::from_ymd(-4713, 11, 24);
    let latest_chrono_date = naive::MAX_DATE;
    let num_days_representable = latest_chrono_date.signed_duration_since(earliest_pg_date).num_days();
    earliest_pg_date + Duration::days(days as i64 % num_days_representable)
}

#[cfg(feature = "mysql")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    let earliest_mysql_date = NaiveDate::from_ymd(1000, 01, 01);
    let latest_mysql_date = NaiveDate::from_ymd(9999, 12, 31);
    let num_days_representable = latest_mysql_date.signed_duration_since(earliest_mysql_date).num_days();
    earliest_mysql_date + Duration::days(days as i64 % num_days_representable)
}

#[cfg(feature = "sqlite")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    let earliest_sqlite_date = naive::MIN_DATE;
    let latest_sqlite_date = naive::MAX_DATE;
    let num_days_representable = latest_sqlite_date.signed_duration_since(earliest_sqlite_date).num_days();
    earliest_sqlite_date + Duration::days(days as i64 % num_days_representable)
}

#[cfg(feature = "postgres")]
mod unstable_types {
    use super::{quickcheck, test_type_round_trips};
    use std::time::*;

    fn strip_nanosecond_precision(time: SystemTime) -> SystemTime {
        match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => time - Duration::new(0, duration.subsec_nanos() % 1000),
            Err(e) => time + Duration::new(0, e.duration().subsec_nanos() % 1000),
        }
    }

    test_round_trip!(systemtime_roundtrips, Timestamp, SystemTime, strip_nanosecond_precision);
}
