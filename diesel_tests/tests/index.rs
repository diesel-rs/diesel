#[cfg(feature = "postgres")]
#[diesel_test_helper::test]
fn test_array_index() {
    use diesel::dsl::array_append;
    use diesel::sql_types::{Array, Integer};
    use diesel::{PgArrayExpressionMethods, RunQueryDsl};
    let connection = &mut crate::schema::connection();
    let result = diesel::select(array_append::<Array<_>, Integer, _, _>(vec![1, 2, 3], 4).index(4))
        .get_result::<i32>(connection)
        .unwrap();

    assert_eq!(4, result);
}
