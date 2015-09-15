use super::schema::*;
use {types, QuerySource};

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
    let actual_data: Vec<_> = connection.query_all(&users::table)
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
    let actual_users: Vec<_> = connection.query_all(&users::table)
        .unwrap().collect();
    assert_eq!(expected_users, actual_users);
}

#[test]
fn with_safe_select() {
    use tests::schema::users::columns::*;
    use tests::schema::users::table as users;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_id = users.select(id);
    // This should fail type checking, and we should add a test to ensure
    // it continues to fail to compile.
    // let select_id = users.select(posts::id);
    let select_name = users.select(name);
    let ids: Vec<_> = connection.query_all(&select_id)
        .unwrap().collect();
    let names: Vec<String> = connection.query_all(&select_name)
        .unwrap().collect();
    // This should fail type checking, and we should add a test to ensure
    // it continues to fail to compile.
    // let names: Vec<String> = connection.query_all(&select_id)
    //     .unwrap().collect();

    assert_eq!(vec![1, 2], ids);
    assert_eq!(vec!["Sean".to_string(), "Tess".to_string()], names);
}

#[test]
fn selecting_multiple_columns() {
    use tests::schema::users::columns::*;
    use tests::schema::users::table as users;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name, hair_color) VALUES ('Jim', 'Black'), ('Bob', 'Brown')")
        .unwrap();

    let source = users.select((name, hair_color));
    // This should fail type checking, and we should add a test to ensure
    // it continues to fail to compile.
    // let source = users.select((posts::title, posts::user_id));
    // let source = users.select((posts::title, name));
    let expected_data = vec![
        ("Jim".to_string(), Some("Black".to_string())),
        ("Bob".to_string(), Some("Brown".to_string())),
    ];
    let actual_data: Vec<_> = connection.query_all(&source)
        .unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn selecting_multiple_columns_into_struct() {
    use tests::schema::users::columns::*;
    use tests::schema::users::table as users;

    let connection = connection();
    setup_users_table(&connection);
    connection.execute("INSERT INTO users (name, hair_color) VALUES ('Jim', 'Black'), ('Bob', 'Brown')")
        .unwrap();

    let source = users.select((name, hair_color));
    let expected_data = vec![
        NewUser::new("Jim", Some("Black")),
        NewUser::new("Bob", Some("Brown")),
    ];
    let actual_data: Vec<_> = connection.query_all(&source)
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
    let get_count = || connection.query_one::<_, i64>(&select_count).unwrap();
    // This should fail type checking, and we should add a test to ensure
    // it continues to fail to compile.
    // let actual_count = connection.query_one::<_, String>(&select_count).unwrap();

    assert_eq!(Some(2), get_count());

    connection.execute("INSERT INTO users (name) VALUES ('Jim')")
        .unwrap();

    assert_eq!(Some(3), get_count());
}
