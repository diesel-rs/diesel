use diesel::*;
use schema::*;

#[test]
fn adding_literal_to_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![2, 3];
    let data = users.select(id + 1).load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data = vec![3, 4];
    let data = users.select(id + 2).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
#[cfg(not(feature = "sqlite"))] // FIXME: Does SQLite provide a way to detect overflow?
fn overflow_returns_an_error_but_does_not_panic() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let query_result = users.select(id + i32::max_value()).load::<i32>(&connection);
    assert!(
        query_result.is_err(),
        "Integer overflow should have returned an error"
    );
}

#[test]
fn adding_column_to_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![2, 4];
    let data = users.select(id + id).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn adding_multiple_times() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![4, 5];
    let data = users.select(id + 1 + 2).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn subtracting_literal_from_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![0, 1];
    let data = users.select(id - 1).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn adding_then_subtracting() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![2, 3];
    let data = users.select(id + 2 - 1).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn multiplying_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![3, 6];
    let data = users.select(id * 3).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn dividing_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![0, 1];
    let data = users.select(id / 2).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn test_adding_nullables() {
    use schema::nullable_table::dsl::*;
    let connection = connection_with_nullable_table_data();

    let expected_data = vec![None, None, Some(2), Some(3), Some(2)];
    let data = nullable_table.select(value + Some(1)).load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data: Vec<Option<i32>> = vec![None; 5];
    let data = nullable_table
        .select(value + None as Option<i32>)
        .load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data = vec![None, None, Some(2), Some(4), Some(2)];
    let data = nullable_table.select(value + value).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn test_subtracting_nullables() {
    use schema::nullable_table::dsl::*;
    let connection = connection_with_nullable_table_data();

    let expected_data = vec![None, None, Some(0), Some(1), Some(0)];
    let data = nullable_table.select(value - Some(1)).load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data: Vec<Option<i32>> = vec![None; 5];
    let data = nullable_table
        .select(value - None as Option<i32>)
        .load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data = vec![None, None, Some(0), Some(0), Some(0)];
    let data = nullable_table.select(value - value).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn test_multiplying_nullables() {
    use schema::nullable_table::dsl::*;
    let connection = connection_with_nullable_table_data();

    let expected_data = vec![None, None, Some(3), Some(6), Some(3)];
    let data = nullable_table.select(value * Some(3)).load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data: Vec<Option<i32>> = vec![None; 5];
    let data = nullable_table
        .select(value * None as Option<i32>)
        .load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data = vec![None, None, Some(1), Some(4), Some(1)];
    let data = nullable_table.select(value * value).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn test_dividing_nullables() {
    use schema::nullable_table::dsl::*;
    let connection = connection_with_nullable_table_data();

    let expected_data = vec![None, None, Some(0), Some(1), Some(0)];
    let data = nullable_table.select(value / Some(2)).load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data: Vec<Option<i32>> = vec![None; 5];
    let data = nullable_table
        .select(value / None as Option<i32>)
        .load(&connection);
    assert_eq!(Ok(expected_data), data);

    let expected_data = vec![None, None, Some(1), Some(1), Some(1)];
    let data = nullable_table.select(value / value).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn mix_and_match_all_numeric_ops() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    connection
        .execute(
            "INSERT INTO users (id, name) VALUES
        (3, 'Jim'), (4, 'Bob')",
        )
        .unwrap();

    let expected_data = vec![4, 6, 7, 9];
    let data = users.select(id * 3 / 2 + 4 - 1).load(&connection);
    assert_eq!(Ok(expected_data), data);
}

#[test]
fn precedence_with_parens_is_maintained() {
    use diesel::sql_types::Integer;

    let x = select((2.into_sql::<Integer>() + 3) * 4).get_result::<i32>(&connection());
    assert_eq!(Ok(20), x);
}
