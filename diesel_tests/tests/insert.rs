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

    batch_insert(new_users, users, &connection);
    let actual_users = users.load::<User>(&connection).unwrap();

    let expected_users = vec![
        User { id: actual_users[0].id, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: actual_users[1].id, name: "Tess".to_string(), hair_color: None },
    ];
    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(not(feature = "sqlite"))]
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
#[cfg(not(feature = "sqlite"))]
fn batch_insert_with_defaults() {
    use schema::users::table as users;
    let connection = connection();
    connection.execute("DROP TABLE users").unwrap();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
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
fn insert_with_defaults() {
    use schema::users::table as users;
    use schema_dsl::*;

    let connection = connection();
    connection.execute("DROP TABLE users").unwrap();
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
fn insert_returning_count_returns_number_of_rows_inserted() {
    use schema::users::table as users;
    let connection = connection();
    connection.execute("DROP TABLE users").unwrap();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
    let new_users: &[_] = &[
        BaldUser { name: "Sean".to_string() },
        BaldUser { name: "Tess".to_string() },
    ];
    let count = batch_insert(new_users, users, &connection);
    let second_count = insert(&BaldUser { name: "Guy".to_string() }).into(users).execute(&connection).unwrap();

    assert_eq!(2, count);
    assert_eq!(1, second_count);
}

#[insertable_into(users)]
struct BaldUser {
    name: String,
}

#[insertable_into(users)]
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
    batch_insert(new_users, users, &connection);

    let actual_users = users.load::<User>(&connection).unwrap();
    let expected_users = vec![
        User::new(actual_users[0].id, "Sean"),
        User::new(actual_users[1].id, "Tess"),
    ];

    assert_eq!(expected_users, actual_users);
}

#[test]
fn delete_records() {
    use schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    let deleted_rows = delete(users.filter(name.eq("Sean"))).execute(&connection);

    assert_eq!(Ok(1), deleted_rows);

    let num_users = users.count().first(&connection);

    assert_eq!(Ok(1), num_users);
}
