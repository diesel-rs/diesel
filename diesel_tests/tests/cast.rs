extern crate chrono;

use crate::schema::*;
use diesel::sql_types::*;
use diesel::*;

#[diesel_test_helper::test]
fn cast_runs() {
    use chrono::NaiveDate;
    use chrono::NaiveTime;

    let connection = &mut connection();

    // (Bool <- Int4)
    diesel::select(true.into_sql::<Bool>().cast::<Integer>())
        .get_result::<i32>(connection)
        .unwrap();

    // (Float4 <- Int4)
    diesel::select(4.into_sql::<Integer>().cast::<Float>())
        .get_result::<f32>(connection)
        .unwrap();

    // (Float4 <- Int8)
    diesel::select(4.into_sql::<BigInt>().cast::<Float>())
        .get_result::<f32>(connection)
        .unwrap();

    // (Float8 <- Float4)
    diesel::select(3.1415f32.into_sql::<Float>().cast::<Double>())
        .get_result::<f64>(connection)
        .unwrap();

    // (Float8 <- Int4)
    diesel::select(4.into_sql::<Integer>().cast::<Double>())
        .get_result::<f64>(connection)
        .unwrap();

    // (Float8 <- Int8)
    diesel::select(4.into_sql::<BigInt>().cast::<Double>())
        .get_result::<f64>(connection)
        .unwrap();

    // (Int8 <- Int4)
    diesel::select(4.into_sql::<Integer>().cast::<BigInt>())
        .get_result::<i64>(connection)
        .unwrap();

    // (Int8 <- Float4)
    diesel::select(3.1415f32.into_sql::<Float>().cast::<BigInt>())
        .get_result::<i64>(connection)
        .unwrap();

    // (Int8 <- Float8)
    diesel::select(3.1415f64.into_sql::<Double>().cast::<BigInt>())
        .get_result::<i64>(connection)
        .unwrap();

    // (Int8 <- Float8)
    diesel::select(3.1415f64.into_sql::<Double>().cast::<BigInt>())
        .get_result::<i64>(connection)
        .unwrap();

    // (Int4 <- Bool)
    diesel::select(true.into_sql::<Bool>().cast::<Integer>())
        .get_result::<i32>(connection)
        .unwrap();

    // (Int4 <- Float4)
    diesel::select(3.1415f32.into_sql::<Float>().cast::<Integer>())
        .get_result::<i32>(connection)
        .unwrap();

    // (Text <- Bool)
    diesel::select(true.into_sql::<Bool>().cast::<Text>())
        .get_result::<String>(connection)
        .unwrap();

    // (Text <- Float4)
    diesel::select(3.1415f32.into_sql::<Float>().cast::<Text>())
        .get_result::<String>(connection)
        .unwrap();

    // (Text <- Float8)
    diesel::select(3.1415f64.into_sql::<Double>().cast::<Text>())
        .get_result::<String>(connection)
        .unwrap();

    // (Text <- Int4)
    diesel::select(4.into_sql::<Integer>().cast::<Text>())
        .get_result::<String>(connection)
        .unwrap();

    // (Text <- Int8)
    diesel::select(4.into_sql::<BigInt>().cast::<Text>())
        .get_result::<String>(connection)
        .unwrap();

    // (Text <- Date)
    diesel::select(
        NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .into_sql::<Date>()
            .cast::<Text>(),
    )
    .get_result::<String>(connection)
    .unwrap();

    // (Text <- Time)
    diesel::select(
        NaiveTime::from_hms_opt(12, 0, 0)
            .unwrap()
            .into_sql::<Time>()
            .cast::<Text>(),
    )
    .get_result::<String>(connection)
    .unwrap();

    // (Int4 <- Int2)
    diesel::select(4.into_sql::<SmallInt>().cast::<Integer>())
        .get_result::<i32>(connection)
        .unwrap();
    // (Int8 <- Int2)
    diesel::select(4.into_sql::<SmallInt>().cast::<BigInt>())
        .get_result::<i64>(connection)
        .unwrap();
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn cast_runs_postgres() {
    use bigdecimal::BigDecimal;

    let connection = &mut connection();

    // (Text <- Json)
    diesel::select(
        serde_json::Value::Bool(true)
            .into_sql::<Json>()
            .cast::<Text>(),
    )
    .get_result::<String>(connection)
    .unwrap();

    // (Text <- Jsonb)
    diesel::select(
        serde_json::Value::Bool(true)
            .into_sql::<Jsonb>()
            .cast::<Text>(),
    )
    .get_result::<String>(connection)
    .unwrap();

    // (Json <- Jsonb)
    diesel::select(
        serde_json::Value::Bool(true)
            .into_sql::<Jsonb>()
            .cast::<Json>(),
    )
    .get_result::<serde_json::Value>(connection)
    .unwrap();

    // (Jsonb <- Json)
    diesel::select(
        serde_json::Value::Bool(true)
            .into_sql::<Jsonb>()
            .cast::<Json>(),
    )
    .get_result::<serde_json::Value>(connection)
    .unwrap();

    // (Numeric <- Int2)
    diesel::select(1.into_sql::<SmallInt>().cast::<Decimal>())
        .get_result::<BigDecimal>(connection)
        .unwrap();
    // (Numeric <- Int4)
    diesel::select(1.into_sql::<Integer>().cast::<Decimal>())
        .get_result::<BigDecimal>(connection)
        .unwrap();
    // (Numeric <- Int8)
    diesel::select(1.into_sql::<BigInt>().cast::<Decimal>())
        .get_result::<BigDecimal>(connection)
        .unwrap();

    // (Text <- Uuid)
    diesel::select(uuid::uuid!("00000000-0000-0000-0000-000000000000").into_sql::<Uuid>().cast::<Text>())
        .get_result::<String>(connection)
        .unwrap();
}
