use super::schema::*;
use yaqb::*;

#[test]
fn selecting_basic_data() {
    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data = vec![
        (1, "Sean".to_string(), None::<String>),
        (2, "Tess".to_string(), None::<String>),
     ];
    let actual_data: Vec<_> = connection.query_all(users::table)
        .unwrap().collect();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn selecting_a_struct() {
    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_users = vec![
        User::new(1, "Sean"),
        User::new(2, "Tess"),
    ];
    let actual_users: Vec<_> = connection.query_all(users::table)
        .unwrap().collect();
    assert_eq!(expected_users, actual_users);
}

#[test]
fn with_safe_select() {
    use schema::users::dsl::*;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_id = users.select(id);
    let select_name = users.select(name);
    let ids: Vec<_> = connection.query_all(select_id)
        .unwrap().collect();
    let names: Vec<String> = connection.query_all(select_name)
        .unwrap().collect();

    assert_eq!(vec![1, 2], ids);
    assert_eq!(vec!["Sean".to_string(), "Tess".to_string()], names);
}

#[test]
fn selecting_multiple_columns() {
    use schema::users::dsl::*;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name, hair_color) VALUES ('Jim', 'Black'), ('Bob', 'Brown')")
        .unwrap();

    let source = users.select((name, hair_color));
    let expected_data = vec![
        ("Jim".to_string(), Some("Black".to_string())),
        ("Bob".to_string(), Some("Brown".to_string())),
    ];
    let actual_data: Vec<_> = connection.query_all(source)
        .unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn selecting_multiple_columns_into_struct() {
    use schema::users::dsl::*;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name, hair_color) VALUES ('Jim', 'Black'), ('Bob', 'Brown')")
        .unwrap();

    let source = users.select((name, hair_color));
    let expected_data = vec![
        NewUser::new("Jim", Some("Black")),
        NewUser::new("Bob", Some("Brown")),
    ];
    let actual_data: Vec<_> = connection.query_all(source)
        .unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn with_select_sql() {
    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_count = users::table.select_sql::<types::BigInt>("COUNT(*)");
    let get_count = || connection.query_one::<_, i64>(select_count.clone()).unwrap();

    assert_eq!(Some(2), get_count());

    connection.execute("INSERT INTO users (name) VALUES ('Jim')")
        .unwrap();

    assert_eq!(Some(3), get_count());
}

#[test]
fn selecting_nullable_followed_by_non_null() {
    use schema::users::dsl::*;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name) VALUES ('Sean')")
        .unwrap();

    let source = users.select((hair_color, name));
    let expected_data = vec![(None::<String>, "Sean".to_string())];
    let data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, data);
}

#[test]
fn selecting_expression_with_bind_param() {
    use schema::users::dsl::*;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let source = users.select(name.eq("Sean".to_string()));
    let expected_data = vec![true, false];
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

table! {
    select {
        id -> Serial,
        join -> Integer,
    }
}

#[test]
fn selecting_columns_and_tables_with_reserved_names() {
    use self::select::dsl::*;

    let connection = connection();
    connection.execute("CREATE TABLE \"select\" (
        id SERIAL PRIMARY KEY,
        \"join\" INTEGER NOT NULL
    )").unwrap();
    connection.execute("INSERT INTO \"select\" (\"join\") VALUES (1), (2), (3)")
        .unwrap();

    let expected_data = vec![(1, 1), (2, 2), (3, 3)];
    let actual_data: Vec<(i32, i32)> = connection.query_all(select)
        .unwrap().collect();
    assert_eq!(expected_data, actual_data);

    let expected_data = vec![1, 2, 3];
    let actual_data: Vec<i32> = connection.query_all(select.select(join))
        .unwrap().collect();
    assert_eq!(expected_data, actual_data);
}
