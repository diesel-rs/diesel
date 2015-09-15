use super::schema::*;

#[test]
fn insert_records() {
    use tests::schema::users::table as users;
    let connection = connection();
    setup_users_table(&connection);

    let new_users = vec![
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];
    let inserted_users: Vec<_> = connection.insert(&users, new_users).unwrap().collect();

    let expected_users = vec![
        User { id: 1, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: 2, name: "Tess".to_string(), hair_color: None },
    ];
    let actual_users: Vec<_> = connection.query_all(&users).unwrap().collect();

    assert_eq!(expected_users, actual_users);
    assert_eq!(expected_users, inserted_users);
}

#[test]
fn insert_with_defaults() {
    use tests::schema::users::table as users;
    let connection = connection();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
    let new_users = vec![
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];
    let inserted_users: Vec<_> = connection.insert(&users, new_users).unwrap().collect();

    let expected_users = vec![
        User { id: 1, name: "Sean".to_string(), hair_color: Some("Black".to_string()) },
        User { id: 2, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
    ];
    let actual_users: Vec<_> = connection.query_all(&users).unwrap().collect();

    assert_eq!(expected_users, actual_users);
    assert_eq!(expected_users, inserted_users);
}

#[test]
fn insert_with_defaults_not_provided() {
    use tests::schema::users::table as users;
    let connection = connection();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )").unwrap();
    let new_users = vec![
        BaldUser { name: "Sean".to_string() },
        BaldUser { name: "Tess".to_string() },
    ];
    let inserted_users: Vec<_> = connection.insert(&users, new_users).unwrap().collect();

    let expected_users = vec![
        User { id: 1, name: "Sean".to_string(), hair_color: Some("Green".to_string()) },
        User { id: 2, name: "Tess".to_string(), hair_color: Some("Green".to_string()) },
    ];
    let actual_users: Vec<_> = connection.query_all(&users).unwrap().collect();

    assert_eq!(expected_users, actual_users);
    assert_eq!(expected_users, inserted_users);
}

struct BaldUser {
    name: String,
}

insertable! {
    BaldUser -> users {
        name -> String,
    }
}
