use schema::*;
use diesel::*;

#[test]
fn find() {
    use schema::users::table as users;

    let connection = connection();

    connection.execute("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .unwrap();

    assert_eq!(Ok(User::new(1, "Sean")), connection.find(users, 1));
    assert_eq!(Ok(User::new(2, "Tess")), connection.find(users, 2));
    assert_eq!(Ok(None::<User>), connection.find(users, 3).optional());
}

table! {
    users_with_name_pk (name) {
        name -> VarChar,
    }
}

#[test]
fn find_with_non_serial_pk() {
    use self::users_with_name_pk::table as users;

    let connection = connection();
    connection.execute("CREATE TABLE users_with_name_pk (name VARCHAR PRIMARY KEY)")
        .unwrap();
    connection.execute("INSERT INTO users_with_name_pk (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    assert_eq!(Ok("Sean".to_string()), connection.find(users, "Sean"));
    assert_eq!(Ok("Tess".to_string()), connection.find(users, "Tess".to_string()));
    assert_eq!(Ok(None::<String>), connection.find(users, "Wibble").optional());
}
