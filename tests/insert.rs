use super::schema::*;

#[test]
fn insert_records() {
    use schema::users::table as users;
    let connection = connection();
    setup_users_table(&connection);

    let new_users = [
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];
    let inserted_users: Vec<_> = connection.insert(&users, &new_users).unwrap().collect();

    let expected_users = vec![
        User { id: 1, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: 2, name: "Tess".to_string(), hair_color: None },
    ];
    let actual_users: Vec<_> = connection.query_all(users).unwrap().collect();

    assert_eq!(expected_users, actual_users);
    assert_eq!(expected_users, inserted_users);
}

#[test]
fn insert_with_defaults() {
    use schema::users::table as users;
    let connection = connection();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
    let new_users = [
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];
    let inserted_users: Vec<_> = connection.insert(&users, &new_users).unwrap().collect();

    let expected_users = vec![
        User { id: 1, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: 2, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
    ];
    let actual_users: Vec<_> = connection.query_all(users).unwrap().collect();

    assert_eq!(expected_users, actual_users);
    assert_eq!(expected_users, inserted_users);
}

#[test]
fn insert_with_defaults_not_provided() {
    use schema::users::table as users;
    let connection = connection();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
    let new_users = [
        BaldUser { name: "Sean".to_string() },
        BaldUser { name: "Tess".to_string() },
    ];
    let inserted_users: Vec<_> = connection.insert(&users, &new_users).unwrap().collect();

    let expected_users = vec![
        User { id: 1, name: "Sean".to_string(), hair_color: Some("Green".to_string()) },
        User { id: 2, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
    ];
    let actual_users: Vec<_> = connection.query_all(users).unwrap().collect();

    assert_eq!(expected_users, actual_users);
    assert_eq!(expected_users, inserted_users);
}

#[test]
fn insert_without_return_returns_number_of_rows_inserted() {
    use schema::users::table as users;
    let connection = connection();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
    let new_users = [
        BaldUser { name: "Sean".to_string() },
        BaldUser { name: "Tess".to_string() },
    ];
    let second_new_users = [BaldUser { name: "Guy".to_string() }];
    let count = connection.insert_without_return(&users, &new_users).unwrap();
    let second_count = connection.insert_without_return(&users, &second_new_users).unwrap();

    assert_eq!(2, count);
    assert_eq!(1, second_count);
}

struct BaldUser {
    name: String,
}

insertable! {
    BaldUser => users {
        name -> String,
    }
}

struct BorrowedUser<'a> {
    name: &'a str,
}

insertable! {
    BorrowedUser<'a> => users {
        name -> &'a str,
    }
}

#[test]
fn insert_borrowed_content() {
    use schema::users::table as users;
    let connection = connection();
    setup_users_table(&connection);
    let new_users = [
        BorrowedUser { name: "Sean" },
        BorrowedUser { name: "Tess" },
    ];
    let inserted_users: Vec<_> = connection.insert(&users, &new_users).unwrap().collect();

    let expected_users = vec![
        User::new(1, "Sean"),
        User::new(2, "Tess"),
    ];
    let actual_users: Vec<_> = connection.query_all(users).unwrap().collect();

    assert_eq!(expected_users, actual_users);
    assert_eq!(expected_users, inserted_users);
}
