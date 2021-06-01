#[cfg(any(feature = "postgres", feature = "mysql"))]
extern crate bigdecimal;
extern crate chrono;

use self::chrono::*;
pub use quickcheck::quickcheck;

pub use crate::schema::{connection_without_transaction, TestConnection};
pub use diesel::data_types::*;
pub use diesel::result::Error;
pub use diesel::serialize::ToSql;
pub use diesel::sql_types::{HasSqlType, SingleValue, SqlType};
pub use diesel::*;

use deserialize::FromSqlRow;
use diesel::expression::{AsExpression, NonAggregate, TypedExpressionType};
use diesel::query_builder::{QueryFragment, QueryId};
#[cfg(feature = "postgres")]
use std::collections::Bound;

pub fn test_type_round_trips<ST, T>(value: T) -> bool
where
    ST: QueryId + SqlType + TypedExpressionType + SingleValue + 'static,
    <TestConnection as Connection>::Backend: HasSqlType<ST>,
    T: AsExpression<ST>
        + FromSqlRow<ST, <TestConnection as Connection>::Backend>
        + PartialEq
        + Clone
        + ::std::fmt::Debug
        + 'static,
    <T as AsExpression<ST>>::Expression: SelectableExpression<(), SqlType = ST>
        + NonAggregate
        + QueryFragment<<TestConnection as Connection>::Backend>
        + QueryId,
{
    let connection = &mut connection_without_transaction();
    let query = select(value.clone().into_sql::<ST>());
    let result = query.get_result::<T>(connection);
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
            if e.message() == "invalid byte sequence for encoding \"UTF8\": 0x00" =>
        {
            true
        }
        Err(e) => panic!("Query failed: {:?}", e),
    }
}

pub fn id<A>(a: A) -> A {
    a
}

macro_rules! test_round_trip {
    ($test_name:ident, $sql_type:ty, $tpe:ty) => {
        test_round_trip!($test_name, $sql_type, $tpe, id);
    };

    ($test_name:ident, $sql_type:ty, $tpe:ty, $map_fn:ident) => {
        #[test]
        fn $test_name() {
            use diesel::sql_types::*;

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
    };
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
    test_round_trip!(
        naive_datetime_roundtrips,
        Timestamp,
        (i64, u32),
        mk_naive_datetime
    );
}

#[cfg(feature = "postgres")]
mod pg_types {
    extern crate ipnetwork;
    extern crate uuid;

    use self::bigdecimal::BigDecimal;
    use super::*;

    test_round_trip!(date_roundtrips, Date, PgDate);
    test_round_trip!(time_roundtrips, Time, PgTime);
    test_round_trip!(timestamp_roundtrips, Timestamp, PgTimestamp);
    test_round_trip!(interval_roundtrips, Interval, PgInterval);
    test_round_trip!(numeric_roundtrips, Numeric, PgNumeric);
    test_round_trip!(money_roundtrips, Money, PgMoney);
    test_round_trip!(
        naive_datetime_roundtrips,
        Timestamp,
        (i64, u32),
        mk_naive_datetime
    );
    test_round_trip!(naive_time_roundtrips, Time, (u32, u32), mk_naive_time);
    test_round_trip!(naive_date_roundtrips, Date, u32, mk_naive_date);
    test_round_trip!(datetime_roundtrips, Timestamptz, (i64, u32), mk_datetime);
    test_round_trip!(
        naive_datetime_roundtrips_tz,
        Timestamptz,
        (i64, u32),
        mk_naive_datetime
    );
    test_round_trip!(
        uuid_roundtrips,
        Uuid,
        (u32, u16, u16, (u8, u8, u8, u8, u8, u8, u8, u8)),
        mk_uuid
    );
    test_round_trip!(array_of_int_roundtrips, Array<Integer>, Vec<i32>);
    test_round_trip!(array_of_bigint_roundtrips, Array<BigInt>, Vec<i64>);
    test_round_trip!(array_of_dynamic_size_roundtrips, Array<Text>, Vec<String>);
    test_round_trip!(
        array_of_nullable_roundtrips,
        Array<Nullable<Text>>,
        Vec<Option<String>>
    );
    test_round_trip!(
        macaddr_roundtrips,
        MacAddr,
        (u8, u8, u8, u8, u8, u8),
        mk_macaddr
    );
    test_round_trip!(cidr_v4_roundtrips, Cidr, (u8, u8, u8, u8), mk_ipv4);
    test_round_trip!(
        cidr_v6_roundtrips,
        Cidr,
        (u16, u16, u16, u16, u16, u16, u16, u16),
        mk_ipv6
    );
    test_round_trip!(inet_v4_roundtrips, Inet, (u8, u8, u8, u8), mk_ipv4);
    test_round_trip!(
        inet_v6_roundtrips,
        Inet,
        (u16, u16, u16, u16, u16, u16, u16, u16),
        mk_ipv6
    );
    test_round_trip!(bigdecimal_roundtrips, Numeric, (i64, u64), mk_bigdecimal);
    test_round_trip!(int4range_roundtrips, Range<Int4>, (i32, i32), mk_bounds);
    test_round_trip!(int8range_roundtrips, Range<Int8>, (i64, i64), mk_bounds);
    test_round_trip!(
        daterange_roundtrips,
        Range<Date>,
        (u32, u32),
        mk_date_bounds
    );
    test_round_trip!(
        numrange_roundtrips,
        Range<Numeric>,
        (i64, u64, i64, u64),
        mk_num_bounds
    );
    test_round_trip!(
        tsrange_roundtrips,
        Range<Timestamp>,
        (i64, u32, i64, u32),
        mk_ts_bounds
    );
    test_round_trip!(
        tstzrange_roundtrips,
        Range<Timestamptz>,
        (i64, u32, i64, u32),
        mk_tstz_bounds
    );

    test_round_trip!(json_roundtrips, Json, SerdeWrapper, mk_serde_json);
    test_round_trip!(jsonb_roundtrips, Jsonb, SerdeWrapper, mk_serde_json);

    fn mk_uuid(data: (u32, u16, u16, (u8, u8, u8, u8, u8, u8, u8, u8))) -> self::uuid::Uuid {
        let a = data.3;
        let b = [a.0, a.1, a.2, a.3, a.4, a.5, a.6, a.7];
        uuid::Uuid::from_fields(data.0, data.1, data.2, &b).unwrap()
    }

    fn mk_macaddr(data: (u8, u8, u8, u8, u8, u8)) -> [u8; 6] {
        [data.0, data.1, data.2, data.3, data.4, data.5]
    }

    fn mk_ipv4(data: (u8, u8, u8, u8)) -> ipnetwork::IpNetwork {
        use std::net::Ipv4Addr;
        let ip = Ipv4Addr::new(data.0, data.1, data.2, data.3);
        ipnetwork::IpNetwork::V4(ipnetwork::Ipv4Network::new(ip, 32).unwrap())
    }

    fn mk_ipv6(data: (u16, u16, u16, u16, u16, u16, u16, u16)) -> ipnetwork::IpNetwork {
        use std::net::Ipv6Addr;
        let ip = Ipv6Addr::new(
            data.0, data.1, data.2, data.3, data.4, data.5, data.6, data.7,
        );
        ipnetwork::IpNetwork::V6(ipnetwork::Ipv6Network::new(ip, 128).unwrap())
    }

    fn mk_bounds<T: Ord + PartialEq>(data: (T, T)) -> (Bound<T>, Bound<T>) {
        if data.0 == data.1 {
            // This is invalid but we don't have a way to say that to quickcheck
            return (Bound::Included(data.0), Bound::Unbounded);
        }

        if data.0 < data.1 {
            (Bound::Included(data.0), Bound::Excluded(data.1))
        } else {
            (Bound::Included(data.1), Bound::Excluded(data.0))
        }
    }

    fn mk_date_bounds(data: (u32, u32)) -> (Bound<NaiveDate>, Bound<NaiveDate>) {
        let d1 = mk_naive_date(data.0);
        let d2 = mk_naive_date(data.1);
        mk_bounds((d1, d2))
    }

    fn mk_num_bounds(data: (i64, u64, i64, u64)) -> (Bound<BigDecimal>, Bound<BigDecimal>) {
        let b1 = mk_bigdecimal((data.0, data.1));
        let b2 = mk_bigdecimal((data.2, data.3));
        mk_bounds((b1, b2))
    }

    fn mk_ts_bounds(data: (i64, u32, i64, u32)) -> (Bound<NaiveDateTime>, Bound<NaiveDateTime>) {
        let ts1 = mk_naive_datetime((data.0, data.1));
        let ts2 = mk_naive_datetime((data.2, data.3));
        mk_bounds((ts1, ts2))
    }

    fn mk_tstz_bounds(data: (i64, u32, i64, u32)) -> (Bound<DateTime<Utc>>, Bound<DateTime<Utc>>) {
        let tstz1 = mk_datetime((data.0, data.1));
        let tstz2 = mk_datetime((data.2, data.3));
        mk_bounds((tstz1, tstz2))
    }

    pub fn mk_datetime(data: (i64, u32)) -> DateTime<Utc> {
        DateTime::from_utc(mk_naive_datetime(data), Utc)
    }
}

#[cfg(feature = "mysql")]
mod mysql_types {
    use super::*;

    test_round_trip!(i8_roundtrips, TinyInt, i8);
    test_round_trip!(
        naive_datetime_roundtrips,
        Timestamp,
        (i64, u32),
        mk_naive_timestamp
    );
    test_round_trip!(
        naive_datetime_roundtrips_to_datetime,
        Datetime,
        (i64, u32),
        mk_naive_datetime
    );
    test_round_trip!(naive_time_roundtrips, Time, (u32, u32), mk_naive_time);
    test_round_trip!(naive_date_roundtrips, Date, u32, mk_naive_date);
    test_round_trip!(bigdecimal_roundtrips, Numeric, (i64, u64), mk_bigdecimal);
    test_round_trip!(u16_roundtrips, Unsigned<SmallInt>, u16);
    test_round_trip!(u32_roundtrips, Unsigned<Integer>, u32);
    test_round_trip!(u64_roundtrips, Unsigned<BigInt>, u64);
    test_round_trip!(json_roundtrips, Json, SerdeWrapper, mk_serde_json);
}

#[cfg(feature = "mysql")]
pub fn mk_naive_timestamp(data: (i64, u32)) -> NaiveDateTime {
    let earliest_mysql_date = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 1);
    let latest_mysql_date = NaiveDate::from_ymd(2038, 1, 19).and_hms(03, 14, 7);
    let mut r = mk_naive_datetime(data);

    loop {
        if r < earliest_mysql_date {
            let diff = earliest_mysql_date - r;
            r = earliest_mysql_date + diff;
        } else if r > latest_mysql_date {
            let diff = r - latest_mysql_date;
            r = earliest_mysql_date + diff;
        } else {
            break;
        }
    }

    r
}

pub fn mk_naive_datetime(data: (i64, u32)) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(data.0, data.1 / 1000)
}

pub fn mk_naive_time(data: (u32, u32)) -> NaiveTime {
    NaiveTime::from_num_seconds_from_midnight(data.0, data.1 / 1000)
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn mk_bigdecimal(data: (i64, u64)) -> bigdecimal::BigDecimal {
    format!("{}.{}", data.0, data.1)
        .parse()
        .expect("Could not interpret as bigdecimal")
}

#[cfg(feature = "postgres")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    use self::chrono::naive::MAX_DATE;

    let earliest_pg_date = NaiveDate::from_ymd(-4713, 11, 24);
    let latest_chrono_date = MAX_DATE;
    let num_days_representable = latest_chrono_date
        .signed_duration_since(earliest_pg_date)
        .num_days();
    earliest_pg_date + Duration::days(days as i64 % num_days_representable)
}

#[cfg(feature = "mysql")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    let earliest_mysql_date = NaiveDate::from_ymd(1000, 01, 01);
    let latest_mysql_date = NaiveDate::from_ymd(9999, 12, 31);
    let num_days_representable = latest_mysql_date
        .signed_duration_since(earliest_mysql_date)
        .num_days();
    earliest_mysql_date + Duration::days(days as i64 % num_days_representable)
}

#[cfg(feature = "sqlite")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    use self::chrono::naive::{MAX_DATE, MIN_DATE};

    let earliest_sqlite_date = MIN_DATE;
    let latest_sqlite_date = MAX_DATE;
    let num_days_representable = latest_sqlite_date
        .signed_duration_since(earliest_sqlite_date)
        .num_days();
    earliest_sqlite_date + Duration::days(days as i64 % num_days_representable)
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
#[derive(Clone, Debug)]
struct SerdeWrapper(serde_json::Value);

#[cfg(any(feature = "postgres", feature = "mysql"))]
impl quickcheck::Arbitrary for SerdeWrapper {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        SerdeWrapper(arbitrary_serde(g, 0))
    }
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn arbitrary_serde<G: quickcheck::Gen>(g: &mut G, depth: usize) -> serde_json::Value {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    match g.gen_range(0, if depth > 0 { 4 } else { 6 }) {
        0 => serde_json::Value::Null,
        1 => serde_json::Value::Bool(g.gen()),
        2 => {
            // don't use floats here
            // comparing floats is complicated
            let n: i32 = g.gen();
            serde_json::Value::Number(n.into())
        }
        3 => {
            let len = g.gen_range(0, 15);
            let s: String = g.sample_iter(Alphanumeric).take(len).collect();
            serde_json::Value::String(s)
        }
        4 => {
            let len = g.gen_range(0, 15);
            let values = (0..len).map(|_| arbitrary_serde(g, depth + 1)).collect();
            serde_json::Value::Array(values)
        }
        5 => {
            let fields = g.gen_range(1, 5);
            let map = (0..fields)
                .map(|_| {
                    let len = g.gen_range(0, 5);
                    let name = g.sample_iter(Alphanumeric).take(len).collect();
                    (name, arbitrary_serde(g, depth + 1))
                })
                .collect();
            serde_json::Value::Object(map)
        }
        _ => unimplemented!(),
    }
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn mk_serde_json(data: SerdeWrapper) -> serde_json::Value {
    data.0
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

    test_round_trip!(
        systemtime_roundtrips,
        Timestamp,
        SystemTime,
        strip_nanosecond_precision
    );
}
