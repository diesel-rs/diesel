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

#[diesel_test_helper::test]
/// Test Int4 -> Int8
pub async fn test_infallible_cast_int4_to_bigint() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();

    let from = 4i32;
    let to = 4i64;

    let result = diesel::select(
        from.into_sql::<diesel::sql_types::Int4>()
            .cast::<diesel::sql_types::BigInt>(),
    )
    .get_result::<i64>(conn)
    .await
    .unwrap();

    let from = None;
    let to = None;

    let result = diesel::select(
        from.into_sql::<Nullable<diesel::sql_types::Int4>>()
            .cast::<Nullable<diesel::sql_types::BigInt>>(),
    )
    .get_result::<Option<i64>>(conn)
    .await
    .unwrap();

    assert_eq!(result, to);
}
