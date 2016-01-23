use schema::connection;
use diesel::*;
use diesel::backend::Pg;
use diesel::types::*;

#[test]
fn boolean_from_sql() {
    assert_eq!(true, query_single_value::<Bool, bool>("'t'::bool"));
    assert_eq!(false, query_single_value::<Bool, bool>("'f'::bool"));
}

#[test]
fn boolean_treats_null_as_false_when_predicates_return_null() {
    let connection = connection();
    let one = AsExpression::<Nullable<Integer>>::as_expression(Some(1));
    let query = select(one.eq(None::<i32>));
    assert_eq!(Ok(false), query.first(&connection));
}

#[test]
fn boolean_to_sql() {
    assert!(query_to_sql_equality::<Bool, bool>("'t'::bool", true));
    assert!(query_to_sql_equality::<Bool, bool>("'f'::bool", false));
    assert!(!query_to_sql_equality::<Bool, bool>("'t'::bool", false));
    assert!(!query_to_sql_equality::<Bool, bool>("'f'::bool", true));
}

#[test]
fn i16_from_sql() {
    assert_eq!(0, query_single_value::<SmallInt, i16>("0::int2"));
    assert_eq!(-1, query_single_value::<SmallInt, i16>("-1::int2"));
    assert_eq!(1, query_single_value::<SmallInt, i16>("1::int2"));
}

#[test]
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
    assert_eq!(70000, query_single_value::<Integer, i32>("70000"));
}

#[test]
fn i32_to_sql_integer() {
    assert!(query_to_sql_equality::<Integer, i32>("0", 0));
    assert!(query_to_sql_equality::<Integer, i32>("-1", -1));
    assert!(query_to_sql_equality::<Integer, i32>("70000", 70000));
    assert!(!query_to_sql_equality::<Integer, i32>("0", 1));
    assert!(!query_to_sql_equality::<Integer, i32>("70000", 69999));
}

#[test]
fn i64_from_sql() {
    assert_eq!(0, query_single_value::<BigInt, i64>("0::int8"));
    assert_eq!(-1, query_single_value::<BigInt, i64>("-1::int8"));
    assert_eq!(283745982374,
               query_single_value::<BigInt, i64>("283745982374::int8"));
}

#[test]
fn i64_to_sql_bigint() {
    assert!(query_to_sql_equality::<BigInt, i64>("0::int8", 0));
    assert!(query_to_sql_equality::<BigInt, i64>("-1::int8", -1));
    assert!(query_to_sql_equality::<BigInt, i64>("283745982374::int8", 283745982374));
    assert!(!query_to_sql_equality::<BigInt, i64>("0::int8", 1));
    assert!(!query_to_sql_equality::<BigInt, i64>("283745982374::int8", 283745982373));
}

use std::{f32, f64};

#[test]
fn f32_from_sql() {
    assert_eq!(0.0, query_single_value::<Float, f32>("0.0::real"));
    assert_eq!(0.5, query_single_value::<Float, f32>("0.5::real"));
    let nan = query_single_value::<Float, f32>("'NaN'::real");
    assert!(nan.is_nan());
    assert_eq!(f32::INFINITY,
               query_single_value::<Float, f32>("'Infinity'::real"));
    assert_eq!(-f32::INFINITY,
               query_single_value::<Float, f32>("'-Infinity'::real"));
}

#[test]
fn f32_to_sql_float() {
    assert!(query_to_sql_equality::<Float, f32>("0.0::real", 0.0));
    assert!(query_to_sql_equality::<Float, f32>("0.5::real", 0.5));
    assert!(query_to_sql_equality::<Float, f32>("'NaN'::real", f32::NAN));
    assert!(query_to_sql_equality::<Float, f32>("'Infinity'::real", f32::INFINITY));
    assert!(query_to_sql_equality::<Float, f32>("'-Infinity'::real", -f32::INFINITY));
    assert!(!query_to_sql_equality::<Float, f32>("0.0::real", 0.5));
    assert!(!query_to_sql_equality::<Float, f32>("'NaN'::real", 0.0));
    assert!(!query_to_sql_equality::<Float, f32>("'Infinity'::real", -f32::INFINITY));
    assert!(!query_to_sql_equality::<Float, f32>("'-Infinity'::real", 1.0));
}

#[test]
fn f64_from_sql() {
    assert_eq!(0.0, query_single_value::<Double, f64>("0.0::double precision"));
    assert_eq!(0.5, query_single_value::<Double, f64>("0.5::double precision"));
    let nan = query_single_value::<Double, f64>("'NaN'::double precision");
    assert!(nan.is_nan());
    assert_eq!(f64::INFINITY,
               query_single_value::<Double, f64>("'Infinity'::double precision"));
    assert_eq!(-f64::INFINITY,
               query_single_value::<Double, f64>("'-Infinity'::double precision"));
}

#[test]
fn f64_to_sql_float() {
    assert!(query_to_sql_equality::<Double, f64>("0.0::double precision", 0.0));
    assert!(query_to_sql_equality::<Double, f64>("0.5::double precision", 0.5));
    assert!(query_to_sql_equality::<Double, f64>("'NaN'::double precision", f64::NAN));
    assert!(query_to_sql_equality::<Double, f64>("'Infinity'::double precision",
                                                f64::INFINITY));
    assert!(query_to_sql_equality::<Double, f64>("'-Infinity'::double precision",
                                                -f64::INFINITY));
    assert!(!query_to_sql_equality::<Double, f64>("0.0::double precision", 0.5));
    assert!(!query_to_sql_equality::<Double, f64>("'NaN'::double precision", 0.0));
    assert!(!query_to_sql_equality::<Double, f64>("'Infinity'::double precision",
                                                 -f64::INFINITY));
    assert!(!query_to_sql_equality::<Double, f64>("'-Infinity'::double precision", 1.0));
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
    assert!(query_to_sql_equality::<VarChar, String>("'hello'", "hello".to_string()));
    assert!(query_to_sql_equality::<VarChar, String>("'world'", "world".to_string()));
    assert!(!query_to_sql_equality::<VarChar, String>("'hello'", "world".to_string()));
}

#[test]
fn binary_from_sql() {
    let invalid_utf8_bytes = vec![0x1Fu8, 0x8Bu8];
    assert_eq!(invalid_utf8_bytes,
               query_single_value::<Binary, Vec<u8>>("E'\\\\x1F8B'::bytea"));
    assert_eq!(Vec::<u8>::new(),
    query_single_value::<Binary, Vec<u8>>("''::bytea"));
    assert_eq!(vec![0u8],
               query_single_value::<Binary, Vec<u8>>("E'\\\\000'::bytea"));
}

#[test]
fn bytes_to_sql_binary() {
    let invalid_utf8_bytes = vec![0x1Fu8, 0x8Bu8];
    assert!(query_to_sql_equality::<Binary, Vec<u8>>("E'\\\\x1F8B'::bytea",
                                                     invalid_utf8_bytes.clone()));
    assert!(query_to_sql_equality::<Binary, &[u8]>("E'\\\\x1F8B'::bytea",
                                                     &invalid_utf8_bytes));
    assert!(!query_to_sql_equality::<Binary, &[u8]>("''::bytea",
                                                     &invalid_utf8_bytes));
    assert!(query_to_sql_equality::<Binary, Vec<u8>>("''::bytea", Vec::<u8>::new()));
    assert!(query_to_sql_equality::<Binary, Vec<u8>>("E'\\\\000'::bytea", vec![0u8]));
}

#[test]
fn option_from_sql() {
    assert_eq!(Some(true),
    query_single_value::<Nullable<Bool>, Option<bool>>("'t'::bool"));
    assert_eq!(None,
               query_single_value::<Nullable<Bool>, Option<bool>>("NULL"));
    assert_eq!(Some(1),
    query_single_value::<Nullable<Integer>, Option<i32>>("1"));
    assert_eq!(None,
               query_single_value::<Nullable<Integer>, Option<i32>>("NULL"));
    assert_eq!(Some("Hello!".to_string()),
    query_single_value::<Nullable<VarChar>, Option<String>>("'Hello!'"));
    assert_eq!(Some("".to_string()),
    query_single_value::<Nullable<VarChar>, Option<String>>("''"));
    assert_eq!(None,
               query_single_value::<Nullable<VarChar>, Option<String>>("NULL"));
}

#[test]
fn option_to_sql() {
    assert!(query_to_sql_equality::<Nullable<Bool>, Option<bool>>("'t'::bool", Some(true)));
    assert!(!query_to_sql_equality::<Nullable<Bool>, Option<bool>>("'f'::bool", Some(true)));
    assert!(query_to_sql_equality::<Nullable<Bool>, Option<bool>>("NULL", None));
    assert!(!query_to_sql_equality::<Nullable<Bool>, Option<bool>>("NULL::bool", Some(false)));
    assert!(query_to_sql_equality::<Nullable<Integer>, Option<i32>>("1", Some(1)));
    assert!(query_to_sql_equality::<Nullable<Integer>, Option<i32>>("NULL", None));
    assert!(query_to_sql_equality::<Nullable<VarChar>,
            Option<String>>("'Hello!'", Some("Hello!".to_string())));
    assert!(query_to_sql_equality::<Nullable<VarChar>,
            Option<String>>("''", Some("".to_string())));
    assert!(query_to_sql_equality::<Nullable<VarChar>, Option<String>>("NULL", None));
}

#[test]
fn pg_array_from_sql() {
    assert_eq!(vec![true, false, true],
               query_single_value::<Array<Bool>, Vec<bool>>(
                   "ARRAY['t', 'f', 't']::bool[]"));
    assert_eq!(vec![1, 2, 3],
               query_single_value::<Array<Integer>, Vec<i32>>("ARRAY[1, 2, 3]"));
    assert_eq!(vec!["Hello".to_string(), "".to_string(), "world".to_string()],
    query_single_value::<Array<VarChar>, Vec<String>>(
        "ARRAY['Hello', '', 'world']"));
}

#[test]
fn to_sql_array() {
    assert!(query_to_sql_equality::<Array<Bool>, Vec<bool>>(
            "ARRAY['t', 'f', 't']::bool[]", vec![true, false, true]));
    assert!(query_to_sql_equality::<Array<Bool>, &[bool]>(
            "ARRAY['t', 'f', 't']::bool[]", &[true, false, true]));
    assert!(!query_to_sql_equality::<Array<Bool>, &[bool]>(
            "ARRAY['t', 'f', 't']::bool[]", &[false, false, true]));
    assert!(query_to_sql_equality::<Array<Integer>, &[i32]>(
            "ARRAY[1, 2, 3]", &[1, 2, 3]));
    assert!(query_to_sql_equality::<Array<VarChar>, &[&str]>(
            "ARRAY['Hello', '', 'world']::varchar[]", &["Hello", "", "world"]));
}

#[test]
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
fn timestamp_from_sql() {
    use diesel::data_types::PgTimestamp;

    let query = "'2015-11-13 13:26:48.041057-07'::timestamp";
    let expected_value = PgTimestamp(500736408041057);
    assert_eq!(expected_value, query_single_value::<Timestamp, PgTimestamp>(query));
    let query = "'2015-11-13 13:26:49.041057-07'::timestamp";
    let expected_value = PgTimestamp(500736409041057);
    assert_eq!(expected_value, query_single_value::<Timestamp, PgTimestamp>(query));
}

#[test]
fn pg_timestamp_to_sql_timestamp() {
    use diesel::data_types::PgTimestamp;

    let expected_value = "'2015-11-13 13:26:48.041057-07'::timestamp";
    let value = PgTimestamp(500736408041057);
    assert!(query_to_sql_equality::<Timestamp, PgTimestamp>(expected_value, value));
    let expected_value = "'2015-11-13 13:26:49.041057-07'::timestamp";
    let value = PgTimestamp(500736409041057);
    assert!(query_to_sql_equality::<Timestamp, PgTimestamp>(expected_value, value));
    let expected_non_equal_value = "'2015-11-13 13:26:48.041057-07'::timestamp";
    assert!(!query_to_sql_equality::<Timestamp, PgTimestamp>(expected_non_equal_value, value));
}

#[test]
fn pg_numeric_from_sql() {
    use diesel::data_types::PgNumeric;

    let query = "1.0::numeric";
    let expected_value = PgNumeric::Positive {
        digits: vec![1],
        weight: 0,
        scale: 1,
    };
    assert_eq!(expected_value, query_single_value::<Numeric, PgNumeric>(query));
    let query = "-31.0::numeric";
    let expected_value = PgNumeric::Negative {
        digits: vec![31],
        weight: 0,
        scale: 1,
    };
    assert_eq!(expected_value, query_single_value::<Numeric, PgNumeric>(query));
    let query = "'NaN'::numeric";
    let expected_value = PgNumeric::NaN;
    assert_eq!(expected_value, query_single_value::<Numeric, PgNumeric>(query));
}

fn query_single_value<T: NativeSqlType, U: Queryable<T, Pg>>(sql_str: &str) -> U {
    use diesel::expression::dsl::sql;
    let connection = connection();
    select(sql::<T>(sql_str)).first(&connection).unwrap()
}

use std::fmt::Debug;
use diesel::expression::AsExpression;
use diesel::query_builder::QueryFragment;

fn query_to_sql_equality<T, U>(sql_str: &str, value: U) -> bool where
    T: NativeSqlType,
    U: AsExpression<T> + Debug + Clone,
    U::Expression: SelectableExpression<(), T> + QueryFragment<Pg>,
{
    use diesel::expression::dsl::sql;
    let connection = connection();
    let query = select(sql::<T>(sql_str).is_not_distinct_from(value.clone()));
    query.get_result(&connection).expect(&format!("Error comparing {}, {:?}", sql_str, value))
}
