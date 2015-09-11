extern crate yaqb;

use self::yaqb::*;
use self::yaqb::types::*;

#[test]
fn boolean_from_sql() {
    assert_eq!(true, query_single_value::<Bool, bool>("SELECT 't'::bool"));
    assert_eq!(false, query_single_value::<Bool, bool>("SELECT 'f'::bool"));
}

#[test]
fn i16_from_sql() {
    assert_eq!(0, query_single_value::<SmallInt, i16>("SELECT 0::int2"));
    assert_eq!(-1, query_single_value::<SmallInt, i16>("SELECT -1::int2"));
    assert_eq!(1, query_single_value::<SmallInt, i16>("SELECT 1::int2"));
}

#[test]
fn i32_from_sql() {
    assert_eq!(0, query_single_value::<Integer, i32>("SELECT 0"));
    assert_eq!(-1, query_single_value::<Integer, i32>("SELECT -1"));
    assert_eq!(70000, query_single_value::<Integer, i32>("SELECT 70000"));
}

#[test]
fn i64_from_sql() {
    assert_eq!(0, query_single_value::<BigInt, i64>("SELECT 0::int8"));
    assert_eq!(-1, query_single_value::<BigInt, i64>("SELECT -1::int8"));
    assert_eq!(283745982374,
               query_single_value::<BigInt, i64>("SELECT 283745982374::int8"));
}

use std::{f32, f64};

#[test]
fn f32_from_sql() {
    assert_eq!(0.0, query_single_value::<Float, f32>("SELECT 0.0::real"));
    assert_eq!(0.5, query_single_value::<Float, f32>("SELECT 0.5::real"));
    let nan = query_single_value::<Float, f32>("SELECT 'NaN'::real");
    assert!(nan.is_nan());
    assert_eq!(f32::INFINITY,
               query_single_value::<Float, f32>("SELECT 'Infinity'::real"));
    assert_eq!(-f32::INFINITY,
               query_single_value::<Float, f32>("SELECT '-Infinity'::real"));
}

#[test]
fn f64_from_sql() {
    assert_eq!(0.0, query_single_value::<Double, f64>("SELECT 0.0::double precision"));
    assert_eq!(0.5, query_single_value::<Double, f64>("SELECT 0.5::double precision"));
    let nan = query_single_value::<Double, f64>("SELECT 'NaN'::double precision");
    assert!(nan.is_nan());
    assert_eq!(f64::INFINITY,
               query_single_value::<Double, f64>("SELECT 'Infinity'::double precision"));
    assert_eq!(-f64::INFINITY,
               query_single_value::<Double, f64>("SELECT '-Infinity'::double precision"));
}

#[test]
fn string_from_sql() {
    assert_eq!("hello", &query_single_value::<VarChar, String>("SELECT 'hello'"));
    assert_eq!("world", &query_single_value::<VarChar, String>("SELECT 'world'"));
}

#[test]
fn binary_from_sql() {
    let invalid_utf8_bytes = vec![0x1Fu8, 0x8Bu8];
    assert_eq!(invalid_utf8_bytes,
               query_single_value::<Binary, Vec<u8>>("SELECT E'\\\\x1F8B'::bytea"));
    assert_eq!(Vec::<u8>::new(),
    query_single_value::<Binary, Vec<u8>>("SELECT ''::bytea"));
    assert_eq!(vec![0u8],
               query_single_value::<Binary, Vec<u8>>("SELECT E'\\\\000'::bytea"));
}

#[test]
fn option_from_sql() {
    assert_eq!(Some(true),
    query_single_value::<Nullable<Bool>, Option<bool>>("SELECT 't'::bool"));
    assert_eq!(None,
               query_single_value::<Nullable<Bool>, Option<bool>>("SELECT NULL"));
    assert_eq!(Some(1),
    query_single_value::<Nullable<Integer>, Option<i32>>("SELECT 1"));
    assert_eq!(None,
               query_single_value::<Nullable<Integer>, Option<i32>>("SELECT NULL"));
    assert_eq!(Some("Hello!".to_string()),
    query_single_value::<Nullable<VarChar>, Option<String>>("SELECT 'Hello!'"));
    assert_eq!(Some("".to_string()),
    query_single_value::<Nullable<VarChar>, Option<String>>("SELECT ''"));
    assert_eq!(None,
               query_single_value::<Nullable<VarChar>, Option<String>>("SELECT NULL"));
}

#[test]
fn pg_array_from_sql() {
    assert_eq!(vec![true, false, true],
               query_single_value::<Array<Bool>, Vec<bool>>(
                   "SELECT ARRAY['t', 'f', 't']::bool[]"));
    assert_eq!(vec![1, 2, 3],
               query_single_value::<Array<Integer>, Vec<i32>>("SELECT ARRAY[1, 2, 3]"));
    assert_eq!(vec!["Hello".to_string(), "".to_string(), "world".to_string()],
    query_single_value::<Array<VarChar>, Vec<String>>(
        "SELECT ARRAY['Hello', '', 'world']"));
}

#[test]
fn pg_array_containing_null() {
    let query = "SELECT ARRAY['Hello', '', NULL, 'world']";
    let data = query_single_value::<Array<Nullable<VarChar>>, Vec<Option<String>>>(query);
    let expected = vec![
        Some("Hello".to_string()),
        Some("".to_string()),
        None,
        Some("world".to_string()),
    ];
    assert_eq!(expected, data);
}

fn query_single_value<T: NativeSqlType, U: Queriable<T>>(sql: &str) -> U {
    let connection = connection();
    let mut cursor = connection.query_sql::<T, U>(sql)
        .unwrap();
    cursor.nth(0).unwrap()
}

fn connection() -> Connection {
    let connection_url = ::std::env::var("DATABASE_URL").ok()
        .expect("DATABASE_URL must be set in order to run tests");
    let result = Connection::establish(&connection_url).unwrap();
    result.execute("BEGIN").unwrap();
    result
}

