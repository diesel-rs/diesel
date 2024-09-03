use crate::schema::connection;
use crate::schema::*;


#[cfg(feature = "postgres")]
#[test]
fn test_array_index() {
    use diesel::{PgArrayExpressionMethods, RunQueryDsl};
    use diesel::dsl::array_append;
    use diesel::sql_types::{Array,Integer};
    let connection = &mut connection();
    let result = diesel::select(array_append::<Array<_>, Integer, _, _>(vec![1, 2, 3], 4).index(4))
        .get_result::<i32>(connection)
        .unwrap();

    assert_eq!(4, result);
}
