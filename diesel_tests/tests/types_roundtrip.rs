use chrono::*;
use diesel::query_dsl::LoadQuery;
pub use quickcheck::quickcheck;

pub use crate::schema::{connection_without_transaction, TestConnection};
#[cfg(not(feature = "sqlite"))]
pub use diesel::data_types::*;
pub use diesel::result::Error;
pub use diesel::sql_types::{HasSqlType, SingleValue, SqlType};
pub use diesel::*;

use deserialize::FromSqlRow;
use diesel::expression::{AsExpression, TypedExpressionType};
use diesel::query_builder::{AsQuery, QueryId};
#[cfg(feature = "postgres")]
use std::collections::Bound;

pub fn test_type_round_trips<ST, T, F>(value: T, cmp: F) -> bool
where
    F: Fn(&T, &T) -> bool,
    ST: QueryId + SqlType + TypedExpressionType + SingleValue,
    <TestConnection as Connection>::Backend: HasSqlType<ST>,
    T: AsExpression<ST>
        + FromSqlRow<ST, <TestConnection as Connection>::Backend>
        + Queryable<ST, <TestConnection as Connection>::Backend>
        + Clone
        + ::std::fmt::Debug
        + 'static,
    for<'a> dsl::select<<T as AsExpression<ST>>::Expression>:
        AsQuery + LoadQuery<'a, TestConnection, T>,
{
    let connection = &mut connection_without_transaction();
    let query = select(value.clone().into_sql::<ST>());
    let result = query.get_result::<T>(connection);
    match result {
        Ok(res) => {
            if cmp(&value, &res) {
                println!("{value:?}, {res:?}");
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
        Err(e) => {
            panic!("Query failed: {:?} -> value: {:?}", e, value)
        }
    }
}

pub fn id<A>(a: A) -> A {
    a
}

pub fn ne<A: PartialEq<A>>(a: &A, b: &A) -> bool {
    a != b
}

macro_rules! test_round_trip {
    ($test_name:ident, $sql_type:ty, $tpe:ty) => {
        test_round_trip!($test_name, $sql_type, $tpe, id);
    };
    ($test_name: ident, $sql_type: ty, $tpe: ty, $map_fn: ident) => {
        test_round_trip!($test_name, $sql_type, $tpe, $map_fn, ne);
    };
    ($test_name:ident, $sql_type:ty, $tpe:ty, $map_fn:ident, $cmp: expr) => {
        #[test]
        #[allow(clippy::type_complexity)]
        fn $test_name() {
            use diesel::sql_types::*;

            fn round_trip(val: $tpe) -> bool {
                test_type_round_trips::<$sql_type, _, _>($map_fn(val), $cmp)
            }

            fn option_round_trip(val: Option<$tpe>) -> bool {
                let val = val.map($map_fn);
                test_type_round_trips::<Nullable<$sql_type>, _, _>(val, |a, b| match (a, b) {
                    (None, None) => false,
                    (_, None) | (None, _) => true,
                    (Some(a), Some(b)) => $cmp(a, b),
                })
            }

            quickcheck(round_trip as fn($tpe) -> bool);
            quickcheck(option_round_trip as fn(Option<$tpe>) -> bool);
        }
    };
}

#[allow(clippy::float_cmp)]
pub fn f32_ne(a: &f32, b: &f32) -> bool {
    if a.is_nan() && b.is_nan() {
        false
    } else {
        a != b
    }
}

#[allow(clippy::float_cmp)]
pub fn f64_ne(a: &f64, b: &f64) -> bool {
    if a.is_nan() && b.is_nan() {
        false
    } else {
        a != b
    }
}

test_round_trip!(i16_roundtrips, SmallInt, i16);
test_round_trip!(i32_roundtrips, Integer, i32);
test_round_trip!(i64_roundtrips, BigInt, i64);
test_round_trip!(f32_roundtrips, Float, FloatWrapper, mk_f32, f32_ne);
test_round_trip!(f64_roundtrips, Double, DoubleWrapper, mk_f64, f64_ne);
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

    use super::*;
    use bigdecimal::BigDecimal;

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
        mk_pg_naive_datetime
    );
    test_round_trip!(naive_time_roundtrips, Time, (u32, u32), mk_pg_naive_time);
    test_round_trip!(naive_date_roundtrips, Date, u32, mk_naive_date);
    test_round_trip!(datetime_roundtrips, Timestamptz, (i64, u32), mk_datetime);
    test_round_trip!(
        naive_datetime_roundtrips_tz,
        Timestamptz,
        (i64, u32),
        mk_pg_naive_datetime
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
        cidr_v4_roundtrips_ipnet,
        Cidr,
        (u8, u8, u8, u8),
        mk_ipv4_ipnet
    );
    test_round_trip!(
        cidr_v6_roundtrips,
        Cidr,
        (u16, u16, u16, u16, u16, u16, u16, u16),
        mk_ipv6
    );
    test_round_trip!(
        cidr_v6_roundtrips_ipnet,
        Cidr,
        (u16, u16, u16, u16, u16, u16, u16, u16),
        mk_ipv6_ipnet
    );
    test_round_trip!(inet_v4_roundtrips, Inet, (u8, u8, u8, u8), mk_ipv4);
    test_round_trip!(
        inet_v4_roundtrips_ipnet,
        Inet,
        (u8, u8, u8, u8),
        mk_ipv4_ipnet
    );
    test_round_trip!(
        inet_v6_roundtrips,
        Inet,
        (u16, u16, u16, u16, u16, u16, u16, u16),
        mk_ipv6
    );
    test_round_trip!(
        inet_v6_roundtrips_ipnet,
        Inet,
        (u16, u16, u16, u16, u16, u16, u16, u16),
        mk_ipv6_ipnet
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

    test_round_trip!(char_roundtrips, CChar, u8);

    #[allow(clippy::type_complexity)]
    fn mk_uuid(data: (u32, u16, u16, (u8, u8, u8, u8, u8, u8, u8, u8))) -> self::uuid::Uuid {
        let a = data.3;
        let b = [a.0, a.1, a.2, a.3, a.4, a.5, a.6, a.7];
        uuid::Uuid::from_fields(data.0, data.1, data.2, &b)
    }

    fn mk_macaddr(data: (u8, u8, u8, u8, u8, u8)) -> [u8; 6] {
        [data.0, data.1, data.2, data.3, data.4, data.5]
    }

    fn mk_ipv4(data: (u8, u8, u8, u8)) -> ipnetwork::IpNetwork {
        use std::net::Ipv4Addr;
        let ip = Ipv4Addr::new(data.0, data.1, data.2, data.3);
        ipnetwork::IpNetwork::V4(ipnetwork::Ipv4Network::new(ip, 32).unwrap())
    }

    fn mk_ipv4_ipnet(data: (u8, u8, u8, u8)) -> ipnet::IpNet {
        use std::net::Ipv4Addr;
        let ip = Ipv4Addr::new(data.0, data.1, data.2, data.3);
        ipnet::IpNet::V4(ipnet::Ipv4Net::new(ip, 32).unwrap())
    }

    fn mk_ipv6(data: (u16, u16, u16, u16, u16, u16, u16, u16)) -> ipnetwork::IpNetwork {
        use std::net::Ipv6Addr;
        let ip = Ipv6Addr::new(
            data.0, data.1, data.2, data.3, data.4, data.5, data.6, data.7,
        );
        ipnetwork::IpNetwork::V6(ipnetwork::Ipv6Network::new(ip, 128).unwrap())
    }

    fn mk_ipv6_ipnet(data: (u16, u16, u16, u16, u16, u16, u16, u16)) -> ipnet::IpNet {
        use std::net::Ipv6Addr;
        let ip = Ipv6Addr::new(
            data.0, data.1, data.2, data.3, data.4, data.5, data.6, data.7,
        );
        ipnet::IpNet::V6(ipnet::Ipv6Net::new(ip, 128).unwrap())
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
        let ts1 = mk_pg_naive_datetime((data.0, data.1));
        let ts2 = mk_pg_naive_datetime((data.2, data.3));
        mk_bounds((ts1, ts2))
    }

    fn mk_tstz_bounds(data: (i64, u32, i64, u32)) -> (Bound<DateTime<Utc>>, Bound<DateTime<Utc>>) {
        let tstz1 = mk_datetime((data.0, data.1));
        let tstz2 = mk_datetime((data.2, data.3));
        mk_bounds((tstz1, tstz2))
    }

    pub fn mk_pg_naive_datetime(data: (i64, u32)) -> NaiveDateTime {
        // https://www.postgresql.org/docs/current/datatype-datetime.html
        let earliest_pg_date = NaiveDate::from_ymd_opt(-4713, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 1)
            .unwrap();
        // chrono cannot even represent that, so don't worry for now
        //let latest_pg_date = NaiveDate::from_ymd(294276, 31,12).and_hms(23,59,59);
        let mut r = mk_naive_datetime(data);

        loop {
            if r < earliest_pg_date {
                let diff = earliest_pg_date - r;
                r = earliest_pg_date + diff;
            } else {
                break;
            }
        }
        // Strip of nano second precision as postgres
        // does not accurately handle them
        let nanos = (r.nanosecond() / 1000) * 1000;
        r.with_nanosecond(nanos).unwrap()
    }

    pub fn mk_pg_naive_time(data: (u32, u32)) -> NaiveTime {
        let time = mk_naive_time(data);

        // Strip of nano second precision as postgres
        // does not accurately handle them
        let nanos = (time.nanosecond() / 1000) * 1000;
        time.with_nanosecond(nanos).unwrap()
    }

    pub fn mk_datetime(data: (i64, u32)) -> DateTime<Utc> {
        Utc.from_utc_datetime(&mk_pg_naive_datetime(data))
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
        mk_mysql_naive_datetime
    );
    test_round_trip!(naive_time_roundtrips, Time, (u32, u32), mk_mysql_naive_time);
    test_round_trip!(naive_date_roundtrips, Date, u32, mk_naive_date);
    test_round_trip!(
        mysql_time_time_roundtrips,
        Time,
        (u32, u32),
        mk_mysql_time_for_time,
        cmp_mysql_time
    );
    test_round_trip!(
        mysql_time_date_roundtrips,
        Date,
        u32,
        mk_mysql_time_for_date,
        cmp_mysql_time
    );
    test_round_trip!(
        mysql_time_date_time_roundtrips,
        Datetime,
        (i64, u32),
        mk_mysql_time_for_date_time,
        cmp_mysql_time
    );
    test_round_trip!(
        mysql_time_timestamp_roundtrips,
        Timestamp,
        (i64, u32),
        mk_mysql_time_for_timestamp,
        cmp_mysql_time
    );
    test_round_trip!(bigdecimal_roundtrips, Numeric, (i64, u64), mk_bigdecimal);
    test_round_trip!(u16_roundtrips, Unsigned<SmallInt>, u16);
    test_round_trip!(u32_roundtrips, Unsigned<Integer>, u32);
    test_round_trip!(u64_roundtrips, Unsigned<BigInt>, u64);
    test_round_trip!(json_roundtrips, Json, SerdeWrapper, mk_serde_json);

    pub fn mk_mysql_time_for_time(data: (u32, u32)) -> MysqlTime {
        let t = mk_mysql_naive_time(data);
        MysqlTime::new(
            0,
            0,
            0,
            t.hour(),
            t.minute(),
            t.second(),
            t.nanosecond() as _,
            false,
            MysqlTimestampType::MYSQL_TIMESTAMP_TIME,
            0,
        )
    }

    pub fn mk_mysql_time_for_date(data: u32) -> MysqlTime {
        let date = mk_naive_date(data);
        MysqlTime::new(
            date.year() as _,
            date.month(),
            date.day(),
            0,
            0,
            0,
            0,
            false,
            MysqlTimestampType::MYSQL_TIMESTAMP_DATE,
            0,
        )
    }

    pub fn mk_mysql_time_for_date_time(data: (i64, u32)) -> MysqlTime {
        let t = mk_mysql_naive_datetime(data);
        MysqlTime::new(
            t.year() as _,
            t.month(),
            t.day(),
            t.hour(),
            t.minute(),
            t.second(),
            t.and_utc().timestamp_subsec_micros() as _,
            false,
            MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
            0,
        )
    }

    pub fn mk_mysql_time_for_timestamp(data: (i64, u32)) -> MysqlTime {
        let t = mk_naive_timestamp(data);
        MysqlTime::new(
            t.year() as _,
            t.month(),
            t.day(),
            t.hour(),
            t.minute(),
            t.second(),
            t.and_utc().timestamp_subsec_micros() as _,
            false,
            MysqlTimestampType::MYSQL_TIMESTAMP_DATETIME,
            0,
        )
    }

    pub fn mk_mysql_naive_time(data: (u32, u32)) -> NaiveTime {
        let time = mk_naive_time(data);

        // Strip of nano second precision as mysql
        // does not handle them at all
        time.with_nanosecond(0).unwrap()
    }

    pub fn mk_naive_timestamp((mut seconds, nanos): (i64, u32)) -> NaiveDateTime {
        // https://dev.mysql.com/doc/refman/8.0/en/datetime.html
        let earliest_mysql_date = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 1)
            .unwrap();
        let latest_mysql_date = NaiveDate::from_ymd_opt(2038, 1, 19)
            .unwrap()
            .and_hms_opt(3, 14, 7)
            .unwrap();

        if seconds != 0 {
            seconds = earliest_mysql_date.and_utc().timestamp()
                + ((latest_mysql_date.and_utc().timestamp()
                    - earliest_mysql_date.and_utc().timestamp())
                    % seconds);
        } else {
            seconds = earliest_mysql_date.and_utc().timestamp();
        }

        let r = mk_naive_datetime((seconds, nanos));

        // Strip of nano second precision as mysql
        // does not accurately handle them
        let nanos = (r.nanosecond() / 1000) * 1000;
        r.with_nanosecond(nanos).unwrap()
    }

    pub fn mk_mysql_naive_datetime(data: (i64, u32)) -> NaiveDateTime {
        // https://dev.mysql.com/doc/refman/8.0/en/datetime.html
        let earliest_mysql_date = NaiveDate::from_ymd_opt(1000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 1)
            .unwrap();
        let latest_mysql_date = NaiveDate::from_ymd_opt(9999, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
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

        // Strip of nano second precision as mysql
        // does not accurately handle them
        let nanos = (r.nanosecond() / 1000) * 1000;
        r.with_nanosecond(nanos).unwrap()
    }

    fn cmp_mysql_time(this: &MysqlTime, other: &MysqlTime) -> bool {
        let is_eq = this.year == other.year
            && this.month == other.month
            && this.day == other.day
            && this.hour == other.hour
            && this.minute == other.minute
            && this.second == other.second
            && this.second_part == other.second_part
            && this.neg == other.neg
            && this.time_zone_displacement == other.time_zone_displacement;

        !is_eq
    }
}

pub fn mk_naive_datetime((mut secs, mut nano): (i64, u32)) -> NaiveDateTime {
    const MAX_SECONDS: i64 = 86_400;
    const MAX_NANO: u32 = 2_000_000_000;
    const MAX_YEAR: i64 = (i32::MAX >> 13) as i64;
    const MIN_YEAR: i64 = (i32::MIN >> 13) as i64;

    nano /= 1000;

    loop {
        let days = secs / MAX_SECONDS;
        let seconds = secs - (days * MAX_SECONDS);
        let years = (days + 719_163) / 365;

        if days + 719_163 > i32::MAX as i64 {
            secs -= ((days + 719_163) - i32::MAX as i64) * MAX_SECONDS;
            continue;
        }

        if years >= MAX_YEAR {
            secs -= (((years - MAX_YEAR) * 365 - 719_163) * MAX_SECONDS).abs();
            continue;
        }

        if years <= MIN_YEAR {
            secs += (((years - MIN_YEAR) * 365 - 719_163) * MAX_SECONDS).abs();
            continue;
        }

        if seconds >= MAX_SECONDS {
            secs -= MAX_SECONDS;
            continue;
        }

        if nano >= MAX_NANO {
            nano -= MAX_NANO;
            continue;
        }
        break;
    }

    DateTime::from_timestamp(secs, nano).unwrap().naive_utc()
}

pub fn mk_naive_time((mut seconds, mut nano): (u32, u32)) -> NaiveTime {
    const MAX_SECONDS: u32 = 86_400;
    const MAX_NANO: u32 = 2_000_000_000;

    nano /= 1000;

    loop {
        match (seconds >= MAX_SECONDS, nano >= MAX_NANO) {
            (true, true) => {
                seconds -= MAX_SECONDS;
                nano -= MAX_NANO;
            }
            (true, false) => {
                seconds -= MAX_SECONDS;
            }
            (false, true) => nano -= MAX_NANO,
            (false, false) => break,
        }
    }

    NaiveTime::from_num_seconds_from_midnight_opt(seconds, nano).unwrap()
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn mk_bigdecimal(data: (i64, u64)) -> bigdecimal::BigDecimal {
    format!("{}.{}", data.0, data.1)
        .parse()
        .expect("Could not interpret as bigdecimal")
}

#[cfg(feature = "postgres")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    let earliest_pg_date = NaiveDate::from_ymd_opt(-4713, 11, 24).unwrap();
    let latest_chrono_date = NaiveDate::MAX;
    let num_days_representable = latest_chrono_date
        .signed_duration_since(earliest_pg_date)
        .num_days();
    earliest_pg_date + Duration::try_days(days as i64 % num_days_representable).unwrap()
}

#[cfg(feature = "mysql")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    let earliest_mysql_date = NaiveDate::from_ymd_opt(1000, 1, 1).unwrap();
    let latest_mysql_date = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();
    let num_days_representable = latest_mysql_date
        .signed_duration_since(earliest_mysql_date)
        .num_days();
    earliest_mysql_date + Duration::try_days(days as i64 % num_days_representable).unwrap()
}

#[cfg(feature = "sqlite")]
pub fn mk_naive_date(days: u32) -> NaiveDate {
    let earliest_sqlite_date = NaiveDate::MIN;
    let latest_sqlite_date = NaiveDate::MAX;
    let num_days_representable = latest_sqlite_date
        .signed_duration_since(earliest_sqlite_date)
        .num_days();
    earliest_sqlite_date + Duration::try_days(days as i64 % num_days_representable).unwrap()
}

#[derive(Debug, Clone, Copy)]
struct FloatWrapper(f32);

fn mk_f32(f: FloatWrapper) -> f32 {
    f.0
}

impl quickcheck::Arbitrary for FloatWrapper {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut f = f32::arbitrary(g);
        while (cfg!(feature = "sqlite") || cfg!(feature = "mysql")) && f.is_nan() {
            f = f32::arbitrary(g);
        }
        FloatWrapper(f)
    }
}

#[derive(Debug, Clone, Copy)]
struct DoubleWrapper(f64);

fn mk_f64(f: DoubleWrapper) -> f64 {
    f.0
}

impl quickcheck::Arbitrary for DoubleWrapper {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let mut f = f64::arbitrary(g);
        while (cfg!(feature = "sqlite") || cfg!(feature = "mysql")) && f.is_nan() {
            f = f64::arbitrary(g);
        }
        DoubleWrapper(f)
    }
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
#[derive(Clone, Debug)]
struct SerdeWrapper(serde_json::Value);

#[cfg(any(feature = "postgres", feature = "mysql"))]
impl quickcheck::Arbitrary for SerdeWrapper {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        SerdeWrapper(arbitrary_serde(g, 0))
    }
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn arbitrary_serde(g: &mut quickcheck::Gen, depth: usize) -> serde_json::Value {
    use quickcheck::Arbitrary;

    let variant = if depth > 1 {
        &[0, 1, 2, 3] as &[_]
    } else {
        &[0, 1, 2, 3, 4, 5] as &[_]
    };
    match g.choose(variant).expect("Slice is not empty") {
        0 => serde_json::Value::Null,
        1 => serde_json::Value::Bool(bool::arbitrary(g)),
        2 => {
            // don't use floats here
            // comparing floats is complicated
            let n = i32::arbitrary(g);
            serde_json::Value::Number(n.into())
        }
        3 => {
            let s = String::arbitrary(g)
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .collect();
            serde_json::Value::String(s)
        }
        4 => {
            let len = g
                .choose(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15_i8])
                .expect("Slice is not empty");
            let values = (0..*len).map(|_| arbitrary_serde(g, depth + 1)).collect();
            serde_json::Value::Array(values)
        }
        5 => {
            let fields = g
                .choose(&[0, 1, 2, 3, 4, 5, 6_i8])
                .expect("Slice is not empty");
            let map = (0..*fields)
                .map(|_| {
                    let len = g.choose(&[1, 2, 3, 4, 5]).expect("Slice is not empty");
                    let name = String::arbitrary(g)
                        .chars()
                        .filter(|c| c.is_ascii_alphabetic())
                        .take(*len)
                        .collect();
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
    use super::{ne, quickcheck, test_type_round_trips};
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
