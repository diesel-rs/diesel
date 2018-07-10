// FIXME: Review this module to see if we can do these casts in a more backend agnostic way

#[cfg(any(feature = "postgres", feature = "mysql"))]
extern crate bigdecimal;
extern crate chrono;

use diesel::deserialize::FromSql;
#[cfg(feature = "postgres")]
use diesel::pg::Pg;
use diesel::sql_types::*;
use diesel::*;
use schema::*;

use quickcheck::quickcheck;

table! {
    has_timestamps {
        id -> Integer,
        ts -> Timestamp,
    }
}

table! {
    has_time_types(datetime) {
        datetime -> Timestamp,
        date -> Date,
        time -> Time,
    }
}

#[test]
#[cfg(feature = "postgres")]
fn errors_during_deserialization_do_not_panic() {
    use self::chrono::NaiveDateTime;
    use self::has_timestamps::dsl::*;
    use diesel::result::Error::DeserializationError;

    let connection = connection();
    connection
        .execute(
            "CREATE TABLE has_timestamps (
        id SERIAL PRIMARY KEY,
        ts TIMESTAMP NOT NULL
    )",
        )
        .unwrap();
    let valid_pg_date_too_large_for_chrono = "'294276/01/01'";
    connection
        .execute(&format!(
            "INSERT INTO has_timestamps (ts) VALUES ({})",
            valid_pg_date_too_large_for_chrono
        ))
        .unwrap();
    let values = has_timestamps.select(ts).load::<NaiveDateTime>(&connection);

    match values {
        Err(DeserializationError(_)) => {}
        v => panic!("Expected a deserialization error, got {:?}", v),
    }
}

#[test]
#[cfg(feature = "sqlite")]
fn errors_during_deserialization_do_not_panic() {
    use self::chrono::NaiveDateTime;
    use self::has_timestamps::dsl::*;
    use diesel::result::Error::DeserializationError;

    let connection = connection();
    connection
        .execute(
            "CREATE TABLE has_timestamps (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts VARCHAR NOT NULL
    )",
        )
        .unwrap();

    let valid_sqlite_date_too_large_for_chrono = "'294276-01-01 00:00:00'";
    connection
        .execute(&format!(
            "INSERT INTO has_timestamps (ts) VALUES ({})",
            valid_sqlite_date_too_large_for_chrono
        ))
        .unwrap();
    let values = has_timestamps.select(ts).load::<NaiveDateTime>(&connection);

    match values {
        Err(DeserializationError(_)) => {}
        v => panic!("Expected a deserialization error, got {:?}", v),
    }
}

#[test]
#[cfg(feature = "sqlite")]
fn test_chrono_types_sqlite() {
    use self::chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use self::has_time_types;

    #[derive(Queryable, Insertable)]
    #[table_name = "has_time_types"]
    struct NewTimeTypes {
        datetime: NaiveDateTime,
        date: NaiveDate,
        time: NaiveTime,
    }

    let connection = connection();
    connection
        .execute(
            "CREATE TABLE has_time_types (
        datetime DATETIME PRIMARY KEY,
        date DATE,
        time TIME
    )",
        )
        .unwrap();

    let dt = NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11);
    let new_time_types = NewTimeTypes {
        datetime: dt,
        date: dt.date(),
        time: dt.time(),
    };

    insert_into(has_time_types::table)
        .values(&new_time_types)
        .execute(&connection)
        .unwrap();

    let result = has_time_types::table
        .first::<NewTimeTypes>(&connection)
        .unwrap();
    assert_eq!(result.datetime, dt);
    assert_eq!(result.date, dt.date());
    assert_eq!(result.time, dt.time());
}

#[test]
#[cfg(feature = "postgres")]
fn boolean_from_sql() {
    assert_eq!(true, query_single_value::<Bool, bool>("'t'::bool"));
    assert_eq!(false, query_single_value::<Bool, bool>("'f'::bool"));
}

#[test]
#[cfg(feature = "postgres")]
fn boolean_treats_null_as_false_when_predicates_return_null() {
    let connection = connection();
    let one = Some(1).into_sql::<Nullable<Integer>>();
    let query = select(one.eq(None::<i32>));
    assert_eq!(Ok(false), query.first(&connection));
}

#[test]
#[cfg(feature = "postgres")]
fn boolean_to_sql() {
    assert!(query_to_sql_equality::<Bool, bool>("'t'::bool", true));
    assert!(query_to_sql_equality::<Bool, bool>("'f'::bool", false));
    assert!(!query_to_sql_equality::<Bool, bool>("'t'::bool", false));
    assert!(!query_to_sql_equality::<Bool, bool>("'f'::bool", true));
}

#[test]
#[cfg(feature = "postgres")]
fn i16_from_sql() {
    assert_eq!(0, query_single_value::<SmallInt, i16>("0::int2"));
    assert_eq!(-1, query_single_value::<SmallInt, i16>("-1::int2"));
    assert_eq!(1, query_single_value::<SmallInt, i16>("1::int2"));
}

#[test]
#[cfg(feature = "postgres")]
fn i16_to_sql_smallint() {
    assert!(query_to_sql_equality::<SmallInt, i16>("0::int2", 0));
    assert!(query_to_sql_equality::<SmallInt, i16>("-1::int2", -1));
    assert!(query_to_sql_equality::<SmallInt, i16>("1::int2", 1));
    assert!(!query_to_sql_equality::<SmallInt, i16>("0::int2", 1));
    assert!(!query_to_sql_equality::<SmallInt, i16>("-1::int2", 1));
}

#[test]
fn i32_from_sql() {
    assert_eq!(0, query_single_value::<Integer, i32>("0"));
    assert_eq!(-1, query_single_value::<Integer, i32>("-1"));
    assert_eq!(70_000, query_single_value::<Integer, i32>("70000"));
}

#[test]
fn i32_to_sql_integer() {
    assert!(query_to_sql_equality::<Integer, i32>("0", 0));
    assert!(query_to_sql_equality::<Integer, i32>("-1", -1));
    assert!(query_to_sql_equality::<Integer, i32>("70000", 70_000));
    assert!(!query_to_sql_equality::<Integer, i32>("0", 1));
    assert!(!query_to_sql_equality::<Integer, i32>("70000", 69_999));
}

#[test]
#[cfg(feature = "mysql")]
fn u16_to_sql_integer() {
    assert!(query_to_sql_equality::<Unsigned<SmallInt>, u16>(
        "65535", 65535
    ));
    assert!(query_to_sql_equality::<Unsigned<SmallInt>, u16>("0", 0));
    assert!(query_to_sql_equality::<Unsigned<SmallInt>, u16>("1", 1));
    assert!(query_to_sql_equality::<Unsigned<SmallInt>, u16>(
        "7000", 7000
    ));
    assert!(!query_to_sql_equality::<Unsigned<SmallInt>, u16>("0", 1));
    assert!(!query_to_sql_equality::<Unsigned<SmallInt>, u16>(
        "50000", 49999
    ));
    assert!(!query_to_sql_equality::<Unsigned<SmallInt>, u16>(
        "64435", 64434
    ));
}

#[test]
#[cfg(feature = "mysql")]
fn u16_from_sql() {
    assert_eq!(0, query_single_value::<Unsigned<SmallInt>, u16>("0"));
    assert_eq!(
        65535,
        query_single_value::<Unsigned<SmallInt>, u16>("65535")
    );
    assert_ne!(
        65534,
        query_single_value::<Unsigned<SmallInt>, u16>("65535")
    );
    assert_eq!(7000, query_single_value::<Unsigned<SmallInt>, u16>("7000"));
}

#[test]
#[cfg(feature = "mysql")]
fn u32_to_sql_integer() {
    assert!(query_to_sql_equality::<Unsigned<Integer>, u32>(
        "4294967295",
        4294967295
    ));
    assert!(query_to_sql_equality::<Unsigned<Integer>, u32>("0", 0));
    assert!(query_to_sql_equality::<Unsigned<Integer>, u32>("1", 1));
    assert!(query_to_sql_equality::<Unsigned<Integer>, u32>(
        "70000", 70000
    ));
    assert!(!query_to_sql_equality::<Unsigned<Integer>, u32>("0", 1));
    assert!(!query_to_sql_equality::<Unsigned<Integer>, u32>(
        "70000", 69999
    ));
    assert!(!query_to_sql_equality::<Unsigned<Integer>, u32>(
        "4294967295",
        4294967294
    ));
}

#[test]
#[cfg(feature = "mysql")]
fn u32_from_sql() {
    assert_eq!(0, query_single_value::<Unsigned<Integer>, u32>("0"));
    assert_eq!(
        4294967295,
        query_single_value::<Unsigned<Integer>, u32>("4294967295")
    );
    assert_ne!(
        4294967294,
        query_single_value::<Unsigned<Integer>, u32>("4294967295")
    );
    assert_eq!(70000, query_single_value::<Unsigned<Integer>, u32>("70000"));
}

#[test]
#[cfg(feature = "mysql")]
fn u64_to_sql_integer() {
    assert!(query_to_sql_equality::<Unsigned<BigInt>, u64>(
        "18446744073709551615",
        18446744073709551615
    ));
    assert!(query_to_sql_equality::<Unsigned<BigInt>, u64>("0", 0));
    assert!(query_to_sql_equality::<Unsigned<BigInt>, u64>("1", 1));
    assert!(query_to_sql_equality::<Unsigned<BigInt>, u64>(
        "700000", 700000
    ));
    assert!(!query_to_sql_equality::<Unsigned<BigInt>, u64>("0", 1));
    assert!(!query_to_sql_equality::<Unsigned<BigInt>, u64>(
        "70000", 69999
    ));
    assert!(!query_to_sql_equality::<Unsigned<BigInt>, u64>(
        "18446744073709551615",
        18446744073709551614
    ));
}

#[test]
#[cfg(feature = "mysql")]
fn u64_from_sql() {
    assert_eq!(0, query_single_value::<Unsigned<BigInt>, u64>("0"));
    assert_eq!(
        18446744073709551615,
        query_single_value::<Unsigned<BigInt>, u64>("18446744073709551615")
    );
    assert_ne!(
        18446744073709551614,
        query_single_value::<Unsigned<BigInt>, u64>("18446744073709551615")
    );
    assert_eq!(
        700000,
        query_single_value::<Unsigned<BigInt>, u64>("700000")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn i64_from_sql() {
    assert_eq!(0, query_single_value::<BigInt, i64>("0::int8"));
    assert_eq!(-1, query_single_value::<BigInt, i64>("-1::int8"));
    assert_eq!(
        283_745_982_374,
        query_single_value::<BigInt, i64>("283745982374::int8")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn i64_to_sql_bigint() {
    assert!(query_to_sql_equality::<BigInt, i64>("0::int8", 0));
    assert!(query_to_sql_equality::<BigInt, i64>("-1::int8", -1));
    assert!(query_to_sql_equality::<BigInt, i64>(
        "283745982374::int8",
        283_745_982_374
    ));
    assert!(!query_to_sql_equality::<BigInt, i64>("0::int8", 1));
    assert!(!query_to_sql_equality::<BigInt, i64>(
        "283745982374::int8",
        283_745_982_373
    ));
}

use std::{f32, f64};

#[test]
#[cfg(feature = "postgres")]
fn f32_from_sql() {
    assert_eq!(0.0, query_single_value::<Float, f32>("0.0::real"));
    assert_eq!(0.5, query_single_value::<Float, f32>("0.5::real"));
    let nan = query_single_value::<Float, f32>("'NaN'::real");
    assert!(nan.is_nan());
    assert_eq!(
        f32::INFINITY,
        query_single_value::<Float, f32>("'Infinity'::real")
    );
    assert_eq!(
        -f32::INFINITY,
        query_single_value::<Float, f32>("'-Infinity'::real")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn f32_to_sql_float() {
    assert!(query_to_sql_equality::<Float, f32>("0.0::real", 0.0));
    assert!(query_to_sql_equality::<Float, f32>("0.5::real", 0.5));
    assert!(query_to_sql_equality::<Float, f32>("'NaN'::real", f32::NAN));
    assert!(query_to_sql_equality::<Float, f32>(
        "'Infinity'::real",
        f32::INFINITY
    ));
    assert!(query_to_sql_equality::<Float, f32>(
        "'-Infinity'::real",
        -f32::INFINITY
    ));
    assert!(!query_to_sql_equality::<Float, f32>("0.0::real", 0.5));
    assert!(!query_to_sql_equality::<Float, f32>("'NaN'::real", 0.0));
    assert!(!query_to_sql_equality::<Float, f32>(
        "'Infinity'::real",
        -f32::INFINITY
    ));
    assert!(!query_to_sql_equality::<Float, f32>(
        "'-Infinity'::real",
        1.0
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn f64_from_sql() {
    assert_eq!(
        0.0,
        query_single_value::<Double, f64>("0.0::double precision")
    );
    assert_eq!(
        0.5,
        query_single_value::<Double, f64>("0.5::double precision")
    );
    let nan = query_single_value::<Double, f64>("'NaN'::double precision");
    assert!(nan.is_nan());
    assert_eq!(
        f64::INFINITY,
        query_single_value::<Double, f64>("'Infinity'::double precision")
    );
    assert_eq!(
        -f64::INFINITY,
        query_single_value::<Double, f64>("'-Infinity'::double precision")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn f64_to_sql_float() {
    assert!(query_to_sql_equality::<Double, f64>(
        "0.0::double precision",
        0.0
    ));
    assert!(query_to_sql_equality::<Double, f64>(
        "0.5::double precision",
        0.5
    ));
    assert!(query_to_sql_equality::<Double, f64>(
        "'NaN'::double precision",
        f64::NAN
    ));
    assert!(query_to_sql_equality::<Double, f64>(
        "'Infinity'::double precision",
        f64::INFINITY
    ));
    assert!(query_to_sql_equality::<Double, f64>(
        "'-Infinity'::double precision",
        -f64::INFINITY
    ));
    assert!(!query_to_sql_equality::<Double, f64>(
        "0.0::double precision",
        0.5
    ));
    assert!(!query_to_sql_equality::<Double, f64>(
        "'NaN'::double precision",
        0.0
    ));
    assert!(!query_to_sql_equality::<Double, f64>(
        "'Infinity'::double precision",
        -f64::INFINITY
    ));
    assert!(!query_to_sql_equality::<Double, f64>(
        "'-Infinity'::double precision",
        1.0
    ));
}

#[test]
fn string_from_sql() {
    assert_eq!("hello", &query_single_value::<VarChar, String>("'hello'"));
    assert_eq!("world", &query_single_value::<VarChar, String>("'world'"));
}

#[test]
fn str_to_sql_varchar() {
    assert!(query_to_sql_equality::<VarChar, &str>("'hello'", "hello"));
    assert!(query_to_sql_equality::<VarChar, &str>("'world'", "world"));
    assert!(!query_to_sql_equality::<VarChar, &str>("'hello'", "world"));
}

#[test]
fn string_to_sql_varchar() {
    assert!(query_to_sql_equality::<VarChar, String>(
        "'hello'",
        "hello".to_string()
    ));
    assert!(query_to_sql_equality::<VarChar, String>(
        "'world'",
        "world".to_string()
    ));
    assert!(!query_to_sql_equality::<VarChar, String>(
        "'hello'",
        "world".to_string()
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn binary_from_sql() {
    let invalid_utf8_bytes = vec![0x1Fu8, 0x8Bu8];
    assert_eq!(
        invalid_utf8_bytes,
        query_single_value::<Binary, Vec<u8>>("E'\\\\x1F8B'::bytea")
    );
    assert_eq!(
        Vec::<u8>::new(),
        query_single_value::<Binary, Vec<u8>>("''::bytea")
    );
    assert_eq!(
        vec![0u8],
        query_single_value::<Binary, Vec<u8>>("E'\\\\000'::bytea")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn bytes_to_sql_binary() {
    let invalid_utf8_bytes = vec![0x1Fu8, 0x8Bu8];
    assert!(query_to_sql_equality::<Binary, Vec<u8>>(
        "E'\\\\x1F8B'::bytea",
        invalid_utf8_bytes.clone()
    ));
    assert!(query_to_sql_equality::<Binary, &[u8]>(
        "E'\\\\x1F8B'::bytea",
        &invalid_utf8_bytes
    ));
    assert!(!query_to_sql_equality::<Binary, &[u8]>(
        "''::bytea",
        &invalid_utf8_bytes
    ));
    assert!(query_to_sql_equality::<Binary, Vec<u8>>(
        "''::bytea",
        Vec::<u8>::new()
    ));
    assert!(query_to_sql_equality::<Binary, Vec<u8>>(
        "E'\\\\000'::bytea",
        vec![0u8]
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_specific_option_from_sql() {
    assert_eq!(
        Some(true),
        query_single_value::<Nullable<Bool>, Option<bool>>("'t'::bool")
    );
}

#[test]
fn option_from_sql() {
    assert_eq!(
        None,
        query_single_value::<Nullable<Bool>, Option<bool>>("NULL")
    );
    assert_eq!(
        Some(1),
        query_single_value::<Nullable<Integer>, Option<i32>>("1")
    );
    assert_eq!(
        None,
        query_single_value::<Nullable<Integer>, Option<i32>>("NULL")
    );
    assert_eq!(
        Some("Hello!".to_string()),
        query_single_value::<Nullable<VarChar>, Option<String>>("'Hello!'")
    );
    assert_eq!(
        Some("".to_string()),
        query_single_value::<Nullable<VarChar>, Option<String>>("''")
    );
    assert_eq!(
        None,
        query_single_value::<Nullable<VarChar>, Option<String>>("NULL")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_specific_option_to_sql() {
    assert!(query_to_sql_equality::<Nullable<Bool>, Option<bool>>(
        "'t'::bool",
        Some(true)
    ));
    assert!(!query_to_sql_equality::<Nullable<Bool>, Option<bool>>(
        "'f'::bool",
        Some(true)
    ));
    assert!(query_to_sql_equality::<Nullable<Bool>, Option<bool>>(
        "NULL", None
    ));
    assert!(!query_to_sql_equality::<Nullable<Bool>, Option<bool>>(
        "NULL::bool",
        Some(false)
    ));
}

#[test]
fn option_to_sql() {
    assert!(query_to_sql_equality::<Nullable<Integer>, Option<i32>>(
        "1",
        Some(1)
    ));
    assert!(query_to_sql_equality::<Nullable<Integer>, Option<i32>>(
        "NULL", None
    ));
    assert!(query_to_sql_equality::<Nullable<VarChar>, Option<String>>(
        "'Hello!'",
        Some("Hello!".to_string())
    ));
    assert!(query_to_sql_equality::<Nullable<VarChar>, Option<String>>(
        "''",
        Some("".to_string())
    ));
    assert!(query_to_sql_equality::<Nullable<VarChar>, Option<String>>(
        "NULL", None
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_array_from_sql() {
    assert_eq!(
        vec![true, false, true],
        query_single_value::<Array<Bool>, Vec<bool>>("ARRAY['t', 'f', 't']::bool[]")
    );
    assert_eq!(
        vec![1, 2, 3],
        query_single_value::<Array<Integer>, Vec<i32>>("ARRAY[1, 2, 3]")
    );
    assert_eq!(
        vec!["Hello".to_string(), "".to_string(), "world".to_string()],
        query_single_value::<Array<VarChar>, Vec<String>>("ARRAY['Hello', '', 'world']")
    );
}

#[cfg(feature = "postgres")]
#[test]
fn pg_array_from_sql_non_one_lower_bound() {
    assert_eq!(
        vec![true, false, true],
        query_single_value::<Array<Bool>, Vec<bool>>("'[0:2]={t, f, t}'::bool[]")
    );
    assert_eq!(
        vec![true, false, true],
        query_single_value::<Array<Bool>, Vec<bool>>("'[1:3]={t, f, t}'::bool[]")
    );
    assert_eq!(
        vec![true, false, true],
        query_single_value::<Array<Bool>, Vec<bool>>("'[2:4]={t, f, t}'::bool[]")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn to_sql_array() {
    assert!(query_to_sql_equality::<Array<Bool>, Vec<bool>>(
        "ARRAY['t', 'f', 't']::bool[]",
        vec![true, false, true]
    ));
    assert!(query_to_sql_equality::<Array<Bool>, &[bool]>(
        "ARRAY['t', 'f', 't']::bool[]",
        &[true, false, true]
    ));
    assert!(!query_to_sql_equality::<Array<Bool>, &[bool]>(
        "ARRAY['t', 'f', 't']::bool[]",
        &[false, false, true]
    ));
    assert!(query_to_sql_equality::<Array<Integer>, &[i32]>(
        "ARRAY[1, 2, 3]",
        &[1, 2, 3]
    ));
    assert!(query_to_sql_equality::<Array<VarChar>, &[&str]>(
        "ARRAY['Hello', '', 'world']::text[]",
        &["Hello", "", "world"]
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_array_containing_null() {
    let query = "ARRAY['Hello', '', NULL, 'world']";
    let data = query_single_value::<Array<Nullable<VarChar>>, Vec<Option<String>>>(query);
    let expected = vec![
        Some("Hello".to_string()),
        Some("".to_string()),
        None,
        Some("world".to_string()),
    ];
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "postgres")]
fn timestamp_from_sql() {
    use diesel::data_types::PgTimestamp;

    let query = "'2015-11-13 13:26:48.041057-07'::timestamp";
    let expected_value = PgTimestamp(500_736_408_041_057);
    assert_eq!(
        expected_value,
        query_single_value::<Timestamp, PgTimestamp>(query)
    );
    let query = "'2015-11-13 13:26:49.041057-07'::timestamp";
    let expected_value = PgTimestamp(500_736_409_041_057);
    assert_eq!(
        expected_value,
        query_single_value::<Timestamp, PgTimestamp>(query)
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_timestamp_to_sql_timestamp() {
    use diesel::data_types::PgTimestamp;

    let expected_value = "'2015-11-13 13:26:48.041057-07'::timestamp";
    let value = PgTimestamp(500_736_408_041_057);
    assert!(query_to_sql_equality::<Timestamp, PgTimestamp>(
        expected_value,
        value
    ));
    let expected_value = "'2015-11-13 13:26:49.041057-07'::timestamp";
    let value = PgTimestamp(500_736_409_041_057);
    assert!(query_to_sql_equality::<Timestamp, PgTimestamp>(
        expected_value,
        value
    ));
    let expected_non_equal_value = "'2015-11-13 13:26:48.041057-07'::timestamp";
    assert!(!query_to_sql_equality::<Timestamp, PgTimestamp>(
        expected_non_equal_value,
        value
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_numeric_from_sql() {
    use diesel::data_types::PgNumeric;

    let query = "1.0::numeric";
    let expected_value = PgNumeric::Positive {
        digits: vec![1],
        weight: 0,
        scale: 1,
    };
    assert_eq!(
        expected_value,
        query_single_value::<Numeric, PgNumeric>(query)
    );
    let query = "-31.0::numeric";
    let expected_value = PgNumeric::Negative {
        digits: vec![31],
        weight: 0,
        scale: 1,
    };
    assert_eq!(
        expected_value,
        query_single_value::<Numeric, PgNumeric>(query)
    );
    let query = "'NaN'::numeric";
    let expected_value = PgNumeric::NaN;
    assert_eq!(
        expected_value,
        query_single_value::<Numeric, PgNumeric>(query)
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_numeric_bigdecimal_to_sql() {
    use self::bigdecimal::BigDecimal;

    fn correct_rep(integer: u64, decimal: u64) -> bool {
        let expected = format!("{}.{}", integer, decimal);
        let value: BigDecimal = expected.parse().expect("Could not parse to a BigDecimal");
        query_to_sql_equality::<Numeric, BigDecimal>(&expected, value)
    }

    quickcheck(correct_rep as fn(u64, u64) -> bool);

    let test_values = vec![
        "0.1",
        "1.0",
        "141.0",
        "-1.0",
        // Larger than u64
        "18446744073709551616",
        // Powers of 10k (numeric is represented in base 10k)
        "10000",
        "100000000",
        "1.100001",
        "10000.100001",
        "0.00001234",
        "120000.00001234",
        "120001.00001234",
    ];

    for value in test_values {
        let expected = format!("'{}'::numeric", value);
        let value = value.parse::<BigDecimal>().unwrap();
        query_to_sql_equality::<Numeric, _>(&expected, value);
    }
}

#[test]
#[cfg(feature = "mysql")]
fn mysql_numeric_bigdecimal_to_sql() {
    use self::bigdecimal::BigDecimal;

    fn correct_rep(integer: u64, decimal: u64) -> bool {
        let expected = format!("{}.{}", integer, decimal);
        let value: BigDecimal = expected.parse().expect("Could not parse to a BigDecimal");
        query_to_sql_equality::<Numeric, BigDecimal>(&expected, value)
    }

    quickcheck(correct_rep as fn(u64, u64) -> bool);

    let test_values = vec![
        "1.0",
        "141.0",
        "-1.0",
        "10000",
        "100000000",
        "1.100001",
        "10000.100001",
        "0.00001234",
        "120000.00001234",
        "120001.00001234",
    ];

    for value in test_values {
        let expected = format!("cast('{}' as decimal(20, 10))", value);
        let value = value.parse::<BigDecimal>().unwrap();
        query_to_sql_equality::<Numeric, _>(&expected, value);
    }
}

#[test]
#[cfg(feature = "postgres")]
fn pg_numeric_bigdecimal_from_sql() {
    use self::bigdecimal::BigDecimal;

    let values = vec![
        "0.1",
        "1.0",
        "141.0",
        "-1.0",
        // With some more precision
        "4.2000000",
        // Larger than u64
        "18446744073709551616",
        // Powers of 10k (numeric is represented in base 10k)
        "10000",
        "100000000",
        "1.100001",
        "10000.100001",
        "0.00001234",
        "120000.00001234",
        "120001.00001234",
    ];

    for value in values {
        let query = format!("'{}'::numeric", value);
        let expected = value.parse::<BigDecimal>().unwrap();
        assert_eq!(expected, query_single_value::<Numeric, BigDecimal>(&query));
        assert_eq!(
            format!("{}", expected),
            format!("{}", query_single_value::<Numeric, BigDecimal>(&query))
        );
    }
}

#[test]
#[cfg(feature = "mysql")]
fn mysql_numeric_bigdecimal_from_sql() {
    use self::bigdecimal::BigDecimal;

    let query = "cast(1.0 as decimal)";
    let expected_value: BigDecimal = "1.0".parse().expect("Could not parse to a BigDecimal");
    assert_eq!(
        expected_value,
        query_single_value::<Numeric, BigDecimal>(query)
    );

    let query = "cast(141.00 as decimal)";
    let expected_value: BigDecimal = "141.00".parse().expect("Could not parse to a BigDecimal");
    assert_eq!(
        expected_value,
        query_single_value::<Numeric, BigDecimal>(query)
    );

    // Some non standard values:
    let query = "cast(18446744073709551616 as decimal)"; // 2^64; doesn't fit in u64
                                                         // It is mysql, it will trim it even in strict mode
    let expected_value: BigDecimal = "9999999999.00"
        .parse()
        .expect("Could not parse to a BigDecimal");
    assert_eq!(
        expected_value,
        query_single_value::<Numeric, BigDecimal>(query)
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_uuid_from_sql() {
    extern crate uuid;

    let query = "'8a645207-42d6-4d17-82e7-f5e42ede0f67'::uuid";
    let expected_value = uuid::Uuid::parse_str("8a645207-42d6-4d17-82e7-f5e42ede0f67").unwrap();
    assert_eq!(
        expected_value,
        query_single_value::<Uuid, uuid::Uuid>(query)
    );
    let query = "'f94e0e4d-c7b0-405f-9c0e-57b97f4afb58'::uuid";
    let expected_value = uuid::Uuid::parse_str("f94e0e4d-c7b0-405f-9c0e-57b97f4afb58").unwrap();
    assert_eq!(
        expected_value,
        query_single_value::<Uuid, uuid::Uuid>(query)
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_uuid_to_sql_uuid() {
    extern crate uuid;

    let expected_value = "'8a645207-42d6-4d17-82e7-f5e42ede0f67'::uuid";
    let value = uuid::Uuid::parse_str("8a645207-42d6-4d17-82e7-f5e42ede0f67").unwrap();
    assert!(query_to_sql_equality::<Uuid, uuid::Uuid>(
        expected_value,
        value
    ));
    let expected_value = "'f94e0e4d-c7b0-405f-9c0e-57b97f4afb58'::uuid";
    let value = uuid::Uuid::parse_str("f94e0e4d-c7b0-405f-9c0e-57b97f4afb58").unwrap();
    assert!(query_to_sql_equality::<Uuid, uuid::Uuid>(
        expected_value,
        value
    ));
    let expected_non_equal_value = "'8e940686-97a5-4e8b-ac44-64cf3cceea9b'::uuid";
    assert!(!query_to_sql_equality::<Uuid, uuid::Uuid>(
        expected_non_equal_value,
        value
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_macaddress_from_sql() {
    let query = "'08:00:2b:01:02:03'::macaddr";
    let expected_value = [0x08, 0x00, 0x2b, 0x01, 0x02, 0x03];
    assert_eq!(
        expected_value,
        query_single_value::<MacAddr, [u8; 6]>(query)
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_macaddress_to_sql_macaddress() {
    let expected_value = "'08:00:2b:01:02:03'::macaddr";
    let value = [0x08, 0x00, 0x2b, 0x01, 0x02, 0x03];
    assert!(query_to_sql_equality::<MacAddr, [u8; 6]>(
        expected_value,
        value
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_v4address_from_sql() {
    extern crate ipnetwork;
    use std::str::FromStr;

    let query = "'192.168.1.0/24'::cidr";
    let expected_value =
        ipnetwork::IpNetwork::V4(ipnetwork::Ipv4Network::from_str("192.168.1.0/24").unwrap());
    assert_eq!(
        expected_value,
        query_single_value::<Cidr, ipnetwork::IpNetwork>(query)
    );
    let query = "'192.168.1.0/24'::inet";
    let expected_value =
        ipnetwork::IpNetwork::V4(ipnetwork::Ipv4Network::from_str("192.168.1.0/24").unwrap());
    assert_eq!(
        expected_value,
        query_single_value::<Inet, ipnetwork::IpNetwork>(query)
    );
}
#[test]
#[cfg(feature = "postgres")]
fn pg_v6address_from_sql() {
    extern crate ipnetwork;
    use std::str::FromStr;

    let query = "'2001:4f8:3:ba::/64'::cidr";
    let expected_value =
        ipnetwork::IpNetwork::V6(ipnetwork::Ipv6Network::from_str("2001:4f8:3:ba::/64").unwrap());
    assert_eq!(
        expected_value,
        query_single_value::<Cidr, ipnetwork::IpNetwork>(query)
    );
    let query = "'2001:4f8:3:ba::/64'::inet";
    let expected_value =
        ipnetwork::IpNetwork::V6(ipnetwork::Ipv6Network::from_str("2001:4f8:3:ba::/64").unwrap());
    assert_eq!(
        expected_value,
        query_single_value::<Inet, ipnetwork::IpNetwork>(query)
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_v4address_to_sql_v4address() {
    extern crate ipnetwork;
    use std::str::FromStr;

    let expected_value = "'192.168.1'::cidr";
    let value =
        ipnetwork::IpNetwork::V4(ipnetwork::Ipv4Network::from_str("192.168.1.0/24").unwrap());
    assert!(query_to_sql_equality::<Cidr, ipnetwork::IpNetwork>(
        expected_value,
        value
    ));
    let expected_value = "'192.168.1.0/24'::inet";
    let value =
        ipnetwork::IpNetwork::V4(ipnetwork::Ipv4Network::from_str("192.168.1.0/24").unwrap());
    assert!(query_to_sql_equality::<Inet, ipnetwork::IpNetwork>(
        expected_value,
        value
    ));
    let expected_non_equal_value = "'192.168.1.0/23'::inet";
    assert!(!query_to_sql_equality::<Inet, ipnetwork::IpNetwork>(
        expected_non_equal_value,
        value
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_v6address_to_sql_v6address() {
    extern crate ipnetwork;
    use std::str::FromStr;

    let expected_value = "'2001:4f8:3:ba::/64'::cidr";
    let value =
        ipnetwork::IpNetwork::V6(ipnetwork::Ipv6Network::from_str("2001:4f8:3:ba::/64").unwrap());
    assert!(query_to_sql_equality::<Cidr, ipnetwork::IpNetwork>(
        expected_value,
        value
    ));
    let expected_value = "'2001:4f8:3:ba::/64'::cidr";
    let value =
        ipnetwork::IpNetwork::V6(ipnetwork::Ipv6Network::from_str("2001:4f8:3:ba::/64").unwrap());
    assert!(query_to_sql_equality::<Inet, ipnetwork::IpNetwork>(
        expected_value,
        value
    ));
    let expected_non_equal_value = "'2001:4f8:3:ba::/63'::cidr";
    assert!(!query_to_sql_equality::<Inet, ipnetwork::IpNetwork>(
        expected_non_equal_value,
        value
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn pg_json_from_sql() {
    extern crate serde_json;

    let query = "'true'::json";
    let expected_value = serde_json::Value::Bool(true);
    assert_eq!(
        expected_value,
        query_single_value::<Json, serde_json::Value>(query)
    );
}

// See https://stackoverflow.com/q/32843213/12089 for why we don't have a
// pg_json_to_sql_json test.  There's no `'true':json = 'true':json`
// because JSON string representations are ambiguous.  We _do_ have this
// test for `jsonb` values.

#[test]
#[cfg(feature = "postgres")]
fn pg_jsonb_from_sql() {
    extern crate serde_json;

    let query = "'true'::jsonb";
    let expected_value = serde_json::Value::Bool(true);
    assert_eq!(
        expected_value,
        query_single_value::<Jsonb, serde_json::Value>(query)
    );
}

#[test]
#[cfg(feature = "postgres")]
fn pg_jsonb_to_sql_jsonb() {
    extern crate serde_json;

    let expected_value = "'false'::jsonb";
    let value = serde_json::Value::Bool(false);
    assert!(query_to_sql_equality::<Jsonb, serde_json::Value>(
        expected_value,
        value
    ));
}

#[test]
#[cfg(feature = "postgres")]
fn text_array_can_be_assigned_to_varchar_array_column() {
    let conn = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &conn);
    let post = insert_into(posts::table)
        .values(&sean.new_post("Hello", None))
        .get_result::<Post>(&conn)
        .unwrap();

    update(posts::table.find(post.id))
        .set(posts::tags.eq(vec!["tag1", "tag2"]))
        .execute(&conn)
        .unwrap();
    let tags_in_db = posts::table.find(post.id).select(posts::tags).first(&conn);

    assert_eq!(Ok(vec!["tag1".to_string(), "tag2".to_string()]), tags_in_db);
}

#[test]
#[cfg(feature = "postgres")]
fn third_party_crates_can_add_new_types() {
    #[derive(Debug, Clone, Copy, QueryId, SqlType)]
    struct MyInt;

    impl HasSqlType<MyInt> for Pg {
        fn metadata(lookup: &Self::MetadataLookup) -> Self::TypeMetadata {
            <Pg as HasSqlType<Integer>>::metadata(lookup)
        }
    }

    impl FromSql<MyInt, Pg> for i32 {
        fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
            FromSql::<Integer, Pg>::from_sql(bytes)
        }
    }

    assert_eq!(0, query_single_value::<MyInt, i32>("0"));
    assert_eq!(-1, query_single_value::<MyInt, i32>("-1"));
    assert_eq!(70_000, query_single_value::<MyInt, i32>("70000"));
}

fn query_single_value<T, U: Queryable<T, TestBackend>>(sql_str: &str) -> U
where
    TestBackend: HasSqlType<T>,
    T: QueryId + SingleValue,
{
    use diesel::dsl::sql;
    let connection = connection();
    select(sql::<T>(sql_str)).first(&connection).unwrap()
}

use diesel::expression::AsExpression;
use diesel::query_builder::{QueryFragment, QueryId};
use std::fmt::Debug;

fn query_to_sql_equality<T, U>(sql_str: &str, value: U) -> bool
where
    U: AsExpression<T> + Debug + Clone,
    U::Expression: SelectableExpression<(), SqlType = T>,
    U::Expression: QueryFragment<TestBackend> + QueryId,
    T: QueryId + SingleValue,
{
    use diesel::dsl::sql;
    let connection = connection();
    let query = select(
        sql::<T>(sql_str)
            .is_null()
            .and(value.clone().as_expression().is_null())
            .or(sql::<T>(sql_str).eq(value.clone())),
    );
    query
        .get_result(&connection)
        .expect(&format!("Error comparing {}, {:?}", sql_str, value))
}

#[cfg(feature = "postgres")]
#[test]
#[should_panic(expected = "Received more than 4 bytes decoding i32")]
fn debug_check_catches_reading_bigint_as_i32_when_using_raw_sql() {
    use diesel::dsl::sql;
    use diesel::sql_types::Integer;

    let connection = connection();
    users::table
        .select(sql::<Integer>("COUNT(*)"))
        .get_result::<i32>(&connection)
        .unwrap();
}

#[cfg(feature = "postgres")]
#[test]
fn test_range_from_sql() {
    use diesel::dsl::sql;
    use std::collections::Bound;

    let connection = connection();

    let query = "'[1,)'::int4range";
    let expected_value = (Bound::Included(1), Bound::Unbounded);
    assert_eq!(
        expected_value,
        query_single_value::<Range<Int4>, (Bound<i32>, Bound<i32>)>(query)
    );

    let query = "'(1,2]'::int4range";
    let expected_value = (Bound::Included(2), Bound::Excluded(3));
    assert_eq!(
        expected_value,
        query_single_value::<Range<Int4>, (Bound<i32>, Bound<i32>)>(query)
    );

    let query = "SELECT '(1,1]'::int4range";
    assert!(
        sql::<Range<Int4>>(query)
            .load::<(Bound<i32>, Bound<i32>)>(&connection)
            .is_err()
    );
}

#[cfg(feature = "postgres")]
#[test]
fn test_range_to_sql() {
    use std::collections::Bound;

    let expected_value = "'[1,2]'::int4range";
    let value = (Bound::Included(1), Bound::Excluded(3));
    assert!(query_to_sql_equality::<Range<Int4>, (Bound<i32>, Bound<i32>)>(expected_value, value));

    let expected_value = "'(1,2]'::int4range";
    let value = (Bound::Included(2), Bound::Excluded(3));
    assert!(query_to_sql_equality::<Range<Int4>, (Bound<i32>, Bound<i32>)>(expected_value, value));
}

#[cfg(feature = "postgres")]
#[test]
fn test_inserting_ranges() {
    use std::collections::Bound;

    let connection = connection();
    connection
        .execute(
            "CREATE TABLE has_ranges (
                        id SERIAL PRIMARY KEY,
                        nul_range INT4RANGE,
                        range INT4RANGE NOT NULL)",
        )
        .unwrap();
    table!(
        has_ranges(id) {
            id -> Int4,
            nul_range -> Nullable<Range<Int4>>,
            range -> Range<Int4>,
        }
    );

    let value = (Bound::Included(1), Bound::Excluded(3));

    let (_, v1, v2): (i32, Option<(_, _)>, (_, _)) = insert_into(has_ranges::table)
        .values((has_ranges::nul_range.eq(value), has_ranges::range.eq(value)))
        .get_result(&connection)
        .unwrap();
    assert_eq!(v1, Some(value));
    assert_eq!(v2, value);
}
