use schema::*;
use diesel::*;
use diesel::query_builder::update;

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
        .unwrap();
    assert_eq!(Some("black".to_string()), data);

    let command = update(users.filter(id.eq(1))).set(hair_color.eq(None::<String>));
    connection.execute_returning_count(&command).unwrap();

    let data: QueryResult<Option<String>> = users.select(hair_color)
        .filter(id.eq(1))
        .first(&connection);
    assert_eq!(Ok(None::<String>), data);
}

#[test]
fn test_updating_multiple_columns() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let command = update(users.filter(id.eq(1))).set((
        name.eq("Jim"),
        hair_color.eq(Some("black")),
    ));
    connection.execute_returning_count(&command).unwrap();

    let expected_user = User::with_hair_color(1, "Jim", "black");
    let user = connection.find(users, 1);
    assert_eq!(Ok(expected_user), user);
}

#[test]
fn update_returning_struct() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let command = update(users.filter(id.eq(1))).set(hair_color.eq("black"));
    let user = connection.query_one(command);
    let expected_user = User::with_hair_color(1, "Sean", "black");

    assert_eq!(Ok(expected_user), user);
}

#[test]
fn update_with_struct_as_changes() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let changes = NewUser::new("Jim", Some("blue"));
    let command = update(users.filter(id.eq(1))).set(&changes);

    let user = connection.query_one(command);
    let expected_user = User::with_hair_color(1, "Jim", "blue");

    assert_eq!(Ok(expected_user), user);
}

#[test]
fn update_with_struct_does_not_set_primary_key() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let changes = User::with_hair_color(2, "Jim", "blue");
    let command = update(users.filter(id.eq(1))).set(&changes);

    let user = connection.query_one(command);
    let expected_user = User::with_hair_color(1, "Jim", "blue");

    assert_eq!(Ok(expected_user), user);
}

#[test]
fn save_on_struct_with_primary_key_changes_that_struct() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let mut user = User::with_hair_color(1, "Jim", "blue");
    user.save_changes(&connection).unwrap();

    let user_in_db = connection.find(users, 1).unwrap();

    assert_eq!(user, user_in_db);
}
