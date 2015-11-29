use schema::*;
use diesel::types::*;

#[test]
fn bind_params_are_passed_for_null_when_not_inserting() {
    let connection = connection();
    let result = connection.query_sql_params::<Integer, i32, Nullable<Integer>, Option<i32>>(
        "SELECT 1 WHERE $1::integer IS NULL", &None).unwrap().nth(0);
    assert_eq!(Some(1), result);
}
