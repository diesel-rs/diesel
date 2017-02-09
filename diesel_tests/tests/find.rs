use schema::*;
use diesel::*;

#[test]
fn find() {
    use schema::users::table as users;

    let connection = connection();

    connection.execute("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .unwrap();

    assert_eq!(Ok(User::new(1, "Sean")), users.find(1).first(&connection));
    assert_eq!(Ok(User::new(2, "Tess")), users.find(2).first(&connection));
    assert_eq!(Ok(None::<User>), users.find(3).first(&connection).optional());
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
    connection.execute("INSERT INTO users_with_name_pk (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    assert_eq!(Ok(("Sean".to_string(),)), users.find("Sean").first(&connection));
    assert_eq!(Ok(("Tess".to_string(),)), users.find("Tess".to_string()).first(&connection));
    assert_eq!(Ok(None::<(String,)>), users.find("Wibble").first(&connection).optional());
}

#[test]
fn find_with_composite_pk() {
    use schema::followings::dsl::*;

    let first_following = Following { user_id: 1, post_id: 1, email_notifications: true };
    let second_following = Following { user_id: 1, post_id: 2, email_notifications: false };
    let third_following = Following { user_id: 2, post_id: 1, email_notifications: false };

    let connection = connection();
    insert(&vec![first_following, second_following, third_following])
        .into(followings)
        .execute(&connection)
        .unwrap();

    assert_eq!(Ok(first_following), followings.find((1, 1)).first(&connection));
    assert_eq!(Ok(second_following), followings.find((1, 2)).first(&connection));
    assert_eq!(Ok(third_following), followings.find((2, 1)).first(&connection));
    assert_eq!(Ok(None::<Following>), followings.find((2, 2)).first(&connection).optional());
}
