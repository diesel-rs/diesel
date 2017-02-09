use super::schema::*;
use diesel::*;
use schema_dsl::*;

#[test]
fn selecting_basic_data() {
    use schema::users::dsl::*;

    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), None::<String>),
        ("Tess".to_string(), None::<String>),
     ];
    let actual_data: Vec<_> = users
        .select((name, hair_color))
        .load(&connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn selecting_a_struct() {
    use schema::users::dsl::*;
    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_users = vec![
        NewUser::new("Sean", None),
        NewUser::new("Tess", None),
    ];
    let actual_users: Vec<_> = users
        .select((name, hair_color))
        .load(&connection).unwrap();
    assert_eq!(expected_users, actual_users);
}

#[test]
fn with_safe_select() {
    use schema::users::dsl::*;

    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_name = users.select(name);
    let names: Vec<String> = select_name.load(&connection).unwrap();

    assert_eq!(vec!["Sean".to_string(), "Tess".to_string()], names);
}

#[test]
fn with_select_sql() {
    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_count = users::table.select_sql::<types::BigInt>("COUNT(*)");
    let get_count = || select_count.clone().first::<i64>(&connection);

    assert_eq!(Ok(2), get_count());

    connection.execute("INSERT INTO users (name) VALUES ('Jim')")
        .unwrap();

    assert_eq!(Ok(3), get_count());
}

#[test]
fn selecting_nullable_followed_by_non_null() {
    use schema::users::dsl::*;

    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean')")
        .unwrap();

    let source = users.select((hair_color, name));
    let expected_data = vec![(None::<String>, "Sean".to_string())];
    let data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, data);
}

#[test]
fn selecting_expression_with_bind_param() {
    use schema::users::dsl::*;

    let connection = connection();
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let source = users.select(name.eq("Sean".to_string()));
    let expected_data = vec![true, false];
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

table! {
    select {
        id -> Integer,
        join -> Integer,
    }
}

#[test]
#[cfg(not(feature="mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_and_tables_with_reserved_names() {
    use self::select::dsl::*;

    let connection = connection();
    create_table("select", (
        integer("id").primary_key().auto_increment(),
        integer("join").not_null(),
    )).execute(&connection).unwrap();
    connection.execute("INSERT INTO \"select\" (\"join\") VALUES (1), (2), (3)")
        .unwrap();

    let expected_data = vec![(1, 1), (2, 2), (3, 3)];
    let actual_data: Vec<(i32, i32)> = select.load(&connection).unwrap();
    assert_eq!(expected_data, actual_data);

    let expected_data = vec![1, 2, 3];
    let actual_data: Vec<i32> = select.select(join).load(&connection).unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
#[cfg(not(feature="mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_with_different_definition_order() {
    let connection = connection();
    connection.execute("DROP TABLE users").unwrap();
    create_table("users", (
        integer("id").primary_key().auto_increment(),
        string("hair_color"),
        string("name").not_null(),
    )).execute(&connection).unwrap();
    let expected_user = User::with_hair_color(1, "Sean", "black");
    insert(&NewUser::new("Sean", Some("black"))).into(users::table)
        .execute(&connection).unwrap();
    let user_from_select = users::table.first(&connection);

    assert_eq!(Ok(&expected_user), user_from_select.as_ref());
}

#[test]
fn selection_using_subselect() {
    use schema::posts::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let ids: Vec<i32> = users::table.select(users::id).load(&connection).unwrap();
    let query = format!(
        "INSERT INTO posts (user_id, title) VALUES ({}, 'Hello'), ({}, 'World')",
        ids[0], ids[1]);
    connection.execute(&query).unwrap();

    let users = users::table.filter(users::name.eq("Sean")).select(users::id);
    let data: Vec<String> = posts
        .select(title)
        .filter(user_id.eq_any(users))
        .load(&connection).unwrap();

    assert_eq!(vec!["Hello".to_string()], data);
}
