use schema::*;
use yaqb::*;
use yaqb::query_builder::update;

#[test]
fn test_updating_single_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let expected_data = vec!["Sean".to_string(), "Tess".to_string()];
    let data: Vec<String> = users.select(name).load(&connection).unwrap().collect();
    assert_eq!(expected_data, data);

    let command = update(users).set(name.eq("Jim"));
    connection.execute_returning_count(&command).unwrap();

    let expected_data = vec!["Jim".to_string(); 2];
    let data: Vec<String> = users.select(name).load(&connection).unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn test_updating_single_column_of_single_row() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let command = update(users.filter(id.eq(1))).set(name.eq("Jim"));
    connection.execute_returning_count(&command).unwrap();

    let expected_data = vec!["Tess".to_string(), "Jim".to_string()];
    let data: Vec<String> = users.select(name).load(&connection).unwrap().collect();
    assert_eq!(expected_data, data);
}

#[test]
fn test_updating_nullable_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let command = update(users.filter(id.eq(1))).set(hair_color.eq(Some("black")));
    connection.execute_returning_count(&command).unwrap();

    let data: Option<String> = users.select(hair_color)
        .filter(id.eq(1))
        .first(&connection)
        .unwrap().unwrap();
    assert_eq!(Some("black".to_string()), data);

    let command = update(users.filter(id.eq(1))).set(hair_color.eq(None::<String>));
    connection.execute_returning_count(&command).unwrap();

    let data: Option<String> = users.select(hair_color)
        .filter(id.eq(1))
        .first(&connection)
        .unwrap().unwrap();
    assert_eq!(None, data);
}
