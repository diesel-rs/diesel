use super::schema::*;
use diesel::*;
use schema_dsl::*;

#[test]
fn selecting_basic_data() {
    use schema::users::dsl::*;

    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), None::<String>),
        ("Tess".to_string(), None::<String>),
    ];
    let actual_data: Vec<_> = users.select((name, hair_color)).load(&connection).unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn selecting_a_struct() {
    use schema::users::dsl::*;
    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_users = vec![NewUser::new("Sean", None), NewUser::new("Tess", None)];
    let actual_users: Vec<_> = users.select((name, hair_color)).load(&connection).unwrap();
    assert_eq!(expected_users, actual_users);
}

#[test]
fn with_safe_select() {
    use schema::users::dsl::*;

    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_name = users.select(name);
    let names: Vec<String> = select_name.load(&connection).unwrap();

    assert_eq!(vec!["Sean".to_string(), "Tess".to_string()], names);
}

#[test]
fn with_select_sql() {
    use diesel::dsl::sql;

    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_count = users::table.select(sql::<types::BigInt>("COUNT(*)"));
    let get_count = || select_count.clone().first::<i64>(&connection);

    assert_eq!(Ok(2), get_count());

    connection
        .execute("INSERT INTO users (name) VALUES ('Jim')")
        .unwrap();

    assert_eq!(Ok(3), get_count());
}

#[test]
fn selecting_nullable_followed_by_non_null() {
    use schema::users::dsl::*;

    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean')")
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
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let source = users.select(name.eq("Sean".to_string()));
    let expected_data = vec![true, false];
    let actual_data = source.load::<bool>(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

table! {
    select {
        id -> Integer,
        join -> Integer,
    }
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_and_tables_with_reserved_names() {
    use self::select::dsl::*;

    let connection = connection();
    create_table(
        "select",
        (
            integer("id").primary_key().auto_increment(),
            integer("join").not_null(),
        ),
    ).execute(&connection)
        .unwrap();
    connection
        .execute("INSERT INTO \"select\" (\"join\") VALUES (1), (2), (3)")
        .unwrap();

    let expected_data = vec![(1, 1), (2, 2), (3, 3)];
    let actual_data: Vec<(i32, i32)> = select.load(&connection).unwrap();
    assert_eq!(expected_data, actual_data);

    let expected_data = vec![1, 2, 3];
    let actual_data: Vec<i32> = select.select(join).load(&connection).unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_with_different_definition_order() {
    let connection = connection();
    drop_table_cascade(&connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("hair_color"),
            string("name").not_null(),
        ),
    ).execute(&connection)
        .unwrap();
    let expected_user = User::with_hair_color(1, "Sean", "black");
    insert_into(users::table)
        .values(&NewUser::new("Sean", Some("black")))
        .execute(&connection)
        .unwrap();
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
        ids[0],
        ids[1]
    );
    connection.execute(&query).unwrap();

    let users = users::table
        .filter(users::name.eq("Sean"))
        .select(users::id);
    let data: Vec<String> = posts
        .select(title)
        .filter(user_id.eq_any(users))
        .load(&connection)
        .unwrap();

    assert_eq!(vec!["Hello".to_string()], data);
}

table! {
    users_select_for_update {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

#[cfg(not(feature = "sqlite"))]
#[test]
fn select_for_update_locks_selected_rows() {
    use self::users_select_for_update::dsl::*;
    use std::mem::drop;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let conn_1 = connection_without_transaction();
    conn_1
        .execute("DROP TABLE IF EXISTS users_select_for_update")
        .unwrap();
    create_table(
        "users_select_for_update",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color"),
        ),
    ).execute(&conn_1)
        .unwrap();
    conn_1
        .batch_execute(
            "
            -- MySQL locks the whole table without this index
            CREATE UNIQUE INDEX users_select_for_update_name ON users_select_for_update (name);
            INSERT INTO users_select_for_update (name) VALUES ('Sean'), ('Tess');
        ",
        )
        .unwrap();
    conn_1.begin_test_transaction().unwrap();

    let _sean = users_select_for_update
        .for_update()
        .filter(name.eq("Sean"))
        .first::<User>(&conn_1)
        .unwrap();

    let (send, recv) = mpsc::channel();
    let send2 = send.clone();

    let _blocked_thread = thread::spawn(move || {
        let conn_2 = connection();
        update(users_select_for_update.filter(name.eq("Sean")))
            .set(name.eq("Jim"))
            .execute(&conn_2)
            .unwrap();
        send.send("Sean").unwrap();
    });

    let _unblocked_thread = thread::spawn(move || {
        let conn_3 = connection();
        update(users_select_for_update.filter(name.eq("Tess")))
            .set(name.eq("Bob"))
            .execute(&conn_3)
            .unwrap();
        send2.send("Tess").unwrap();
    });

    let timeout = Duration::from_secs(1);
    let next_selected_name = recv.recv_timeout(timeout).unwrap();
    // conn_3 will always complete before conn_2, as conn_2 is blocked
    assert_eq!("Tess", next_selected_name);
    let next_selected_name = recv.recv_timeout(timeout);
    // conn_2 should still be blocked
    assert!(next_selected_name.is_err());
    drop(conn_1);
    let next_selected_name = recv.recv_timeout(timeout).unwrap();
    // Dropping conn_1 unblocks conn_2
    assert_eq!("Sean", next_selected_name);
}

#[test]
fn select_can_be_called_on_query_that_is_valid_subselect_but_invalid_query() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let tess = find_user_by_name("Tess", &connection);
    insert_into(posts::table)
        .values(&vec![
            tess.new_post("Tess", None),
            sean.new_post("Hi", None),
        ])
        .execute(&connection)
        .unwrap();

    let invalid_query_but_valid_subselect = posts::table
        .filter(posts::title.eq(users::name))
        .select(posts::user_id);
    let users_with_post_using_name_as_title = users::table
        .filter(users::id.eq_any(invalid_query_but_valid_subselect))
        .load(&connection);

    assert_eq!(Ok(vec![tess]), users_with_post_using_name_as_title);
}
