use crate::{connection, TestConnection};
use diesel::deserialize::FromSqlRow;
use diesel::expression::{AsExpression, ValidGrouping};
use diesel::prelude::*;
use diesel::query_builder::{NoFromClause, QueryFragment, QueryId};
use diesel::sql_types::{self, HasSqlType, SingleValue};
use diesel_async::{AsyncConnectionCore, RunQueryDsl};
use std::fmt::Debug;

async fn type_check<T, ST>(conn: &mut TestConnection, value: T)
where
    T: Clone
        + AsExpression<ST>
        + FromSqlRow<ST, <TestConnection as AsyncConnectionCore>::Backend>
        + Send
        + PartialEq
        + Debug
        + Clone
        + 'static,
    T::Expression: ValidGrouping<()>
        + SelectableExpression<NoFromClause>
        + QueryFragment<<TestConnection as AsyncConnectionCore>::Backend>
        + QueryId
        + Send,
    <TestConnection as AsyncConnectionCore>::Backend: HasSqlType<ST>,
    ST: SingleValue,
{
    let res = diesel::select(value.clone().into_sql())
        .get_result::<T>(conn)
        .await;

    assert_eq!(Ok(value), res);
}

#[tokio::test]
async fn check_small_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::SmallInt>(conn, 1_i16).await;
    type_check::<_, sql_types::SmallInt>(conn, 1_i16).await;
    type_check::<_, sql_types::SmallInt>(conn, i16::MIN).await;
    type_check::<_, sql_types::SmallInt>(conn, i16::MAX).await;
}

#[tokio::test]
async fn check_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Integer>(conn, 1_i32).await;
    type_check::<_, sql_types::Integer>(conn, -1_i32).await;
    type_check::<_, sql_types::Integer>(conn, i32::MIN).await;
    type_check::<_, sql_types::Integer>(conn, i32::MAX).await;
}

#[tokio::test]
async fn check_big_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::BigInt>(conn, 1_i64).await;
    type_check::<_, sql_types::BigInt>(conn, -1_i64).await;
    type_check::<_, sql_types::BigInt>(conn, i64::MIN).await;
    type_check::<_, sql_types::BigInt>(conn, i64::MAX).await;
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn check_tiny_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::TinyInt>(conn, 1_i8).await;
    type_check::<_, sql_types::TinyInt>(conn, -1_i8).await;
    type_check::<_, sql_types::TinyInt>(conn, i8::MIN).await;
    type_check::<_, sql_types::TinyInt>(conn, i8::MAX).await;

    // test case for https://github.com/weiznich/diesel_async/issues/91
    let res = diesel::dsl::sql::<diesel::sql_types::Bool>("SELECT -1 = ")
        .bind::<sql_types::TinyInt, _>(-1)
        .get_result::<bool>(conn)
        .await
        .unwrap();
    assert!(res);
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn check_unsigned_tiny_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Unsigned<sql_types::TinyInt>>(conn, 1_u8).await;
    type_check::<_, sql_types::Unsigned<sql_types::TinyInt>>(conn, u8::MIN).await;
    type_check::<_, sql_types::Unsigned<sql_types::TinyInt>>(conn, u8::MAX).await;
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn check_unsigned_small_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Unsigned<sql_types::SmallInt>>(conn, 1_u16).await;
    type_check::<_, sql_types::Unsigned<sql_types::SmallInt>>(conn, u16::MIN).await;
    type_check::<_, sql_types::Unsigned<sql_types::SmallInt>>(conn, u16::MAX).await;
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn check_unsigned_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Unsigned<sql_types::Integer>>(conn, 1_u32).await;
    type_check::<_, sql_types::Unsigned<sql_types::Integer>>(conn, u32::MIN).await;
    type_check::<_, sql_types::Unsigned<sql_types::Integer>>(conn, u32::MAX).await;
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn check_unsigned_big_int() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Unsigned<sql_types::BigInt>>(conn, 1_u64).await;
    type_check::<_, sql_types::Unsigned<sql_types::BigInt>>(conn, u64::MIN).await;
    type_check::<_, sql_types::Unsigned<sql_types::BigInt>>(conn, u64::MAX).await;
}

#[tokio::test]
async fn check_bool() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Bool>(conn, false).await;
    type_check::<_, sql_types::Bool>(conn, false).await;
}

#[tokio::test]
async fn check_f32() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Float4>(conn, 1.0_f32).await;
    type_check::<_, sql_types::Float4>(conn, f32::MIN_POSITIVE).await;
    type_check::<_, sql_types::Float4>(conn, f32::MIN).await;
    type_check::<_, sql_types::Float4>(conn, f32::MAX).await;
}

#[tokio::test]
async fn check_f64() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Float8>(conn, 1.0_f64).await;
    type_check::<_, sql_types::Float8>(conn, f64::MIN_POSITIVE).await;
    type_check::<_, sql_types::Float8>(conn, f64::MIN).await;
    type_check::<_, sql_types::Float8>(conn, f64::MAX).await;
}

#[tokio::test]
async fn check_string() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Text>(conn, String::from("Test")).await;
    type_check::<_, sql_types::Text>(conn, String::new()).await;
    type_check::<_, sql_types::Text>(conn, String::from("üöä")).await;
}

#[tokio::test]
async fn check_option() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Nullable<sql_types::Integer>>(conn, Some(42)).await;
    type_check::<_, sql_types::Nullable<sql_types::Integer>>(conn, None::<i32>).await;

    type_check::<_, sql_types::Nullable<sql_types::Text>>(conn, Some(String::new())).await;
    type_check::<_, sql_types::Nullable<sql_types::Text>>(conn, None::<String>).await;
}

#[tokio::test]
async fn test_blob() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Blob>(conn, b"foo".to_vec()).await;
    type_check::<_, sql_types::Blob>(conn, Vec::new()).await;
}

#[tokio::test]
async fn test_timestamp() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Timestamp>(
        conn,
        chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2021, 9, 27).unwrap(),
            chrono::NaiveTime::from_hms_milli_opt(17, 44, 23, 0).unwrap(),
        ),
    )
    .await;
}

#[tokio::test]
async fn test_date() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Date>(conn, chrono::NaiveDate::from_ymd_opt(2021, 9, 27).unwrap())
        .await;
}

#[tokio::test]
async fn test_time() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Time>(
        conn,
        chrono::NaiveTime::from_hms_milli_opt(17, 44, 23, 0).unwrap(),
    )
    .await;
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
#[tokio::test]
async fn test_datetime() {
    let conn = &mut connection().await;
    type_check::<_, sql_types::Datetime>(
        conn,
        chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(2021, 9, 30).unwrap(),
            chrono::NaiveTime::from_hms_milli_opt(12, 6, 42, 0).unwrap(),
        ),
    )
    .await;
}
