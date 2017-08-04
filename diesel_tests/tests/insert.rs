use super::schema::*;
use diesel::*;

#[test]
fn insert_records() {
    use schema::users::table as users;
    let connection = connection();
    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    insert(new_users).into(users).execute(&connection).unwrap();
    let actual_users = users.load::<User>(&connection).unwrap();

    let expected_users = vec![
        User { id: actual_users[0].id, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: actual_users[1].id, name: "Tess".to_string(), hair_color: None },
    ];
    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(not(any(feature="sqlite", feature="mysql")))]
fn insert_records_using_returning_clause() {
    use schema::users::table as users;
    let connection = connection();
    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    let inserted_users = insert(new_users).into(users).get_results::<User>(&connection).unwrap();
    let expected_users = vec![
        User { id: inserted_users[0].id, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: inserted_users[1].id, name: "Tess".to_string(), hair_color: None },
    ];

    assert_eq!(expected_users, inserted_users);
}

#[test]
#[cfg(not(any(feature="sqlite", feature="mysql")))]
fn insert_records_with_custom_returning_clause() {
    use schema::users::dsl::*;

    let connection = connection();
    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    let inserted_users = insert(new_users)
        .into(users)
        .returning((name, hair_color))
        .get_results::<(String, Option<String>)>(&connection)
        .unwrap();
    let expected_users = vec![
        ("Sean".to_string(), Some("Black".to_string())),
        ("Tess".to_string(), None),
    ];

    assert_eq!(expected_users, inserted_users);
}

#[test]
#[cfg(not(feature="mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn batch_insert_with_defaults() {
    use schema::users::table as users;
    use schema_dsl::*;

    let connection = connection();
    drop_table_cascade(&connection, "users");
    create_table("users", (
        integer("id").primary_key().auto_increment(),
        string("name").not_null(),
        string("hair_color").not_null().default("'Green'"),
    )).execute(&connection).unwrap();

    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];
    insert(new_users).into(users).execute(&connection).unwrap();

    let expected_users = vec![
        User { id: 1, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: 2, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
    ];
    let actual_users = users.load(&connection).unwrap();

    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(not(feature="mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn insert_with_defaults() {
    use schema::users::table as users;
    use schema_dsl::*;

    let connection = connection();
    drop_table_cascade(&connection, "users");
    create_table("users", (
        integer("id").primary_key().auto_increment(),
        string("name").not_null(),
        string("hair_color").not_null().default("'Green'"),
    )).execute(&connection).unwrap();
    insert(&NewUser::new("Tess", None)).into(users).execute(&connection).unwrap();

    let expected_users = vec![
        User { id: 1, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
    ];
    let actual_users = users.load(&connection).unwrap();

    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(not(feature="mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn insert_returning_count_returns_number_of_rows_inserted() {
    use schema::users::table as users;
    let connection = connection();
    drop_table_cascade(&connection, "users");
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
    let new_users: &[_] = &[
        BaldUser { name: "Sean".to_string() },
        BaldUser { name: "Tess".to_string() },
    ];
    let count = insert(new_users).into(users).execute(&connection).unwrap();
    let second_count = insert(&BaldUser { name: "Guy".to_string() }).into(users).execute(&connection).unwrap();

    assert_eq!(2, count);
    assert_eq!(1, second_count);
}

#[derive(Insertable)]
#[table_name="users"]
struct BaldUser {
    name: String,
}

#[derive(Insertable)]
#[table_name="users"]
struct BorrowedUser<'a> {
    name: &'a str,
}

#[test]
fn insert_borrowed_content() {
    use schema::users::table as users;
    let connection = connection();
    let new_users: &[_] = &[
        BorrowedUser { name: "Sean" },
        BorrowedUser { name: "Tess" },
    ];
    insert(new_users).into(users).execute(&connection).unwrap();

    let actual_users = users.load::<User>(&connection).unwrap();
    let expected_users = vec![
        User::new(actual_users[0].id, "Sean"),
        User::new(actual_users[1].id, "Tess"),
    ];

    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(feature = "sqlite")]
fn insert_on_conflict_replace() {
    use schema::users::dsl::*;
    use diesel::insert_or_replace;

    let connection = connection_with_sean_and_tess_in_users_table();

    let mut sean = find_user_by_name("Sean", &connection);
    sean.name = "Jim".into();
    insert_or_replace(&sean)
        .into(users)
        .execute(&connection)
        .unwrap();

    let expected_names = vec!["Jim".into(), "Tess".into()];
    let names = users.select(name).order(id).load::<String>(&connection);
    assert_eq!(Ok(expected_names), names);
}

#[test]
fn insert_empty_slice() {
    let connection = connection();

    let inserted_records = insert(&Vec::<NewUser>::new())
        .into(users::table)
        .execute(&connection);

    assert_eq!(Ok(0), inserted_records);
}

#[test]
#[cfg(feature = "postgres")]
fn insert_empty_slice_with_returning() {
    let connection = connection();

    let insert_one = insert(&Vec::<NewUser>::new())
        .into(users::table)
        .get_result::<User>(&connection);
    let insert_all = insert(&Vec::<NewUser>::new())
        .into(users::table)
        .get_results::<User>(&connection);

    assert_eq!(Ok(None), insert_one.optional());
    assert_eq!(Ok(vec![]), insert_all);
}

#[test]
#[cfg(not(feature = "mysql"))]
fn insert_only_default_values() {
    use schema::users::table as users;
    use schema_dsl::*;
    let connection = connection();

    drop_table_cascade(&connection, "users");
    create_table("users", (
        integer("id").primary_key().auto_increment(),
        string("name").not_null().default("'Sean'"),
        string("hair_color").not_null().default("'Green'"),
    )).execute(&connection).unwrap();

    insert_default_values().into(users).execute(&connection);
    assert_eq!(users.load::<User>(&connection), Ok(vec![User { id: 1, name: "Sean".into(), hair_color: Some("Green".into()) }]));
}

#[test]
#[cfg(feature = "postgres")]
fn insert_only_default_values_with_returning() {
    use schema::users::table as users;
    use schema::users::id;
    use schema_dsl::*;
    let connection = connection();

    drop_table_cascade(&connection, "users");
    create_table("users", (
        integer("id").primary_key().auto_increment(),
        string("name").not_null().default("'Sean'"),
        string("hair_color").not_null().default("'Green'"),
    )).execute(&connection).unwrap();
    let result = LoadDsl::get_result::<i32>(insert_default_values().into(users).returning(id), &connection).unwrap();

    assert_eq!(result, 1);
    assert_eq!(users.load::<User>(&connection), Ok(vec![User { id: 1, name: "Sean".into(), hair_color: Some("Green".into()) }]));
}
