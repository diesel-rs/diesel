use schema::*;
use yaqb::*;

#[test]
fn test_adding_literal_to_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![2, 3];
    let data: Vec<_> = users.select(id + 1).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);

    let expected_data = vec![3, 4];
    let data: Vec<_> = users.select(id + 2).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn test_adding_column_to_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![2, 4];
    let data: Vec<_> = users.select(id + id).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn test_adding_multiple_times() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![4, 5];
    let data: Vec<_> = users.select(id + 1 + 2).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}
