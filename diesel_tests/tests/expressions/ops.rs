use schema::*;
use diesel::*;

#[test]
fn adding_literal_to_column() {
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
fn adding_column_to_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![2, 4];
    let data: Vec<_> = users.select(id + id).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn adding_multiple_times() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![4, 5];
    let data: Vec<_> = users.select(id + 1 + 2).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn subtracting_literal_from_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![0, 1];
    let data: Vec<_> = users.select(id - 1).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn adding_then_subtracting() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![2, 3];
    let data: Vec<_> = users.select(id + 2 - 1).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn multiplying_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![3, 6];
    let data: Vec<_> = users.select(id * 3).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn dividing_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![0, 1];
    let data: Vec<_> = users.select(id / 2).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn mix_and_match_all_numeric_ops() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let data = vec![NewUser::new("Jim", None), NewUser::new("Bob", None)];
    connection.insert_returning_count(&users, &data).unwrap();

    let expected_data = vec![4, 6, 7, 9];
    let data: Vec<_> = users.select(id * 3 / 2 + 4 - 1).load(&connection)
        .unwrap().collect();
    assert_eq!(expected_data, data);
}
