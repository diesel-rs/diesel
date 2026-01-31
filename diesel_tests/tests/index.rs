#[cfg(feature = "postgres")]
#[diesel_test_helper::test]
fn test_array_index() {
    use diesel::sql_types::{Array, Integer};
    use diesel::{IntoSql, NullableExpressionMethods, PgArrayExpressionMethods, RunQueryDsl};

    let connection = &mut crate::schema::connection();

    let data = &[1, 2, 3, 4][..];
    let arr = data.into_sql::<Array<Integer>>();

    let result = diesel::select(arr.index(4))
        .get_result::<i32>(connection)
        .unwrap();

    assert_eq!(4, result);

    let result = diesel::select(arr.index_nullable(4))
        .get_result::<Option<i32>>(connection)
        .unwrap();

    assert_eq!(Some(4), result);

    let result = diesel::select(arr.nullable().index(4))
        .get_result::<Option<i32>>(connection)
        .unwrap();

    assert_eq!(Some(4), result);

    let result = diesel::select(arr.nullable().index_nullable(4))
        .get_result::<Option<i32>>(connection)
        .unwrap();

    assert_eq!(Some(4), result);
}

#[cfg(feature = "postgres")]
#[diesel_test_helper::test]
fn test_array_slice() {
    use diesel::sql_types::{Array, Integer};
    use diesel::{IntoSql, NullableExpressionMethods, PgArrayExpressionMethods, RunQueryDsl};

    let connection = &mut crate::schema::connection();

    let data = &[1, 2, 3, 4][..];
    let arr = data.into_sql::<Array<Integer>>();

    let result = diesel::select(arr.slice(3, 4))
        .get_result::<Vec<i32>>(connection)
        .unwrap();

    assert_eq!(vec![3, 4], result);

    let result = diesel::select(arr.slice_nullable(3, 4))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![3, 4]), result);

    let result = diesel::select(arr.nullable().slice(3, 4))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![3, 4]), result);

    let result = diesel::select(arr.nullable().slice_nullable(3, 4))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![3, 4]), result);
}

#[cfg(feature = "postgres")]
#[diesel_test_helper::test]
fn test_array_slice_from() {
    use diesel::sql_types::{Array, Integer};
    use diesel::{IntoSql, NullableExpressionMethods, PgArrayExpressionMethods, RunQueryDsl};

    let connection = &mut crate::schema::connection();

    let data = &[1, 2, 3, 4][..];
    let arr = data.into_sql::<Array<Integer>>();

    let result = diesel::select(arr.slice_from(3))
        .get_result::<Vec<i32>>(connection)
        .unwrap();

    assert_eq!(vec![3, 4], result);

    let result = diesel::select(arr.slice_from_nullable(3))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![3, 4]), result);

    let result = diesel::select(arr.nullable().slice_from(3))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![3, 4]), result);

    let result = diesel::select(arr.nullable().slice_from_nullable(3))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![3, 4]), result);
}

#[cfg(feature = "postgres")]
#[diesel_test_helper::test]
fn test_array_slice_to() {
    use diesel::sql_types::{Array, Integer};
    use diesel::{IntoSql, NullableExpressionMethods, PgArrayExpressionMethods, RunQueryDsl};

    let connection = &mut crate::schema::connection();

    let data = &[1, 2, 3, 4][..];
    let arr = data.into_sql::<Array<Integer>>();

    let result = diesel::select(arr.slice_to(2))
        .get_result::<Vec<i32>>(connection)
        .unwrap();

    assert_eq!(vec![1, 2], result);

    let result = diesel::select(arr.slice_to_nullable(2))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![1, 2]), result);

    let result = diesel::select(arr.nullable().slice_to(2))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![1, 2]), result);

    let result = diesel::select(arr.nullable().slice_to_nullable(2))
        .get_result::<Option<Vec<i32>>>(connection)
        .unwrap();

    assert_eq!(Some(vec![1, 2]), result);
}
