/*casts_impl!(
    (Bool <- Int4),
    (Int4 <- Bool),
    (Int4 <- Float4),
    (Int8 <- Int4),
    (Int8 <- Float4),
    (Int8 <- Float8),
    (Float4 <- Int4),
    (Float4 <- Int8),
    (Float8 <- Float4),
    (Float8 <- Int4),
    (Float8 <- Int8),
    (Decimal <- Bool),
    (Decimal <- Int4),
    (Decimal <- Int8),
    (Decimal <- Float4),
    (Decimal <- Float8),
    (Text <- Bool),
    (Text <- Float4),
    (Text <- Float8),
    (Text <- Int4),
    (Text <- Int8),
    (Text <- Date),
    (Text <- Json),
    (Text <- Jsonb),
    (Text <- Time),
    (Json <- Jsonb),
    (Jsonb <- Json),
    "mysql_backend": (Text <- Datetime),
    "postgres_backend": (Text <- Uuid),

);*/

macro_rules! fallible_casts_impl {
    (
        $(
            $($feature: literal => )? ($to_expr: expr, $from_expr: expr):($to: tt <- $from: tt),
        )+
    ) => {
        $(
            $(#[cfg(feature = $feature)])?
            concat_idents!( test_name = "test_fallible_cast_", $from, "_to_", $to {
                $(#[cfg(feature = $feature)])?
                #[diesel_test_helper::test]
                fn $test_name() {
                    let conn = &mut connection_with_sean_and_tess_in_users_table();

                    let from = $from_expr;
                    let to = $to_expr;

                    let result = diesel::select(
                        from.into_sql::<diesel::sql_types::$from>()
                            .fallible_cast::<diesel::sql_types::$to>(),
                    )
                    .get_result(conn)
                    .await
                    .unwrap();

                    assert_eq!(result, to);
                }
            })
        )+
    };
}

fallible_casts_impl!(
    (3i32, 3i64):(Int4 <- Int8),
    (3i32, 3.3f64):(Int4 <- Float8),
    (3i32, "3"):(Int4 <- Text),
    (3i32, "3.0"):(Int4 <- Decimal),
    (3i64, "3"):(Int8 <- Text),
    (3i64, "3.0"):(Int8 <- Decimal),
    (3.3f32, 3.3f64):(Float4 <- Float8),
    (3.3f32, "3.3"):(Float4 <- Text),
    (3.3f32, "3.3"):(Float4 <- Decimal),
    (3.3f64, "3.3"):(Float8 <- Text),
    (3.3f64, "3.3"):(Float8 <- Decimal),
    (serde_json::json!({"key":"value"}), r#"{"key":"value"}"#):(Json <- Text),
    (serde_json::json!({"key":"value"}), r#"{"key":"value"}"#):(Jsonb <- Text),
    (true, "true"):(Bool <- Text),
    (chrono::NaiveDate::from_ymd_opt(2025, 9, 18).unwrap(), "2025-09-18"):(Date <- Text),
    (chrono::NaiveTime::from_hms_opt(12, 34, 56).unwrap(), "12:34:56"):(Time <- Text),
    (bigdecimal::BigDecimal::from(3.3), "3.3"):(Decimal <- Text),
    "mysql_backend" => (chrono::NaiveDateTime::new(
        chrono::NaiveDate::from_ymd_opt(2025, 9, 18).unwrap(),
        chrono::NaiveTime::from_hms_opt(12, 34, 56).unwrap()
    ), "2025-09-18 12:34:56"):(Datetime <- Text),
    "postgres_backend" => (uuid::Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap(), "123e4567-e89b-12d3-a456-426614174000"):(Uuid <- Text),
);

#[diesel_test_helper::test]
/// Test Text -> Int4
pub async fn test_fallible_cast_text_to_int4() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();

    let from = "3";
    let to = 3;

    let result = diesel::select(
        from.into_sql::<diesel::sql_types::Text>()
            .fallible_cast::<diesel::sql_types::Int4>(),
    )
    .get_result::<i32>(conn)
    .await
    .unwrap();

    let from = None;
    let to = None;

    let result = diesel::select(
        from.into_sql::<Nullable<diesel::sql_types::Text>>()
            .fallible_cast::<Nullable<diesel::sql_types::Int4>>(),
    )
    .get_result::<Option<i32>>(conn)
    .await
    .unwrap();

    assert_eq!(result, to);
}
