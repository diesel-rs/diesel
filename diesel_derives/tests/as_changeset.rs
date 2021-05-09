use diesel::expression::{bound::Bound, grouped::Grouped, operators};
use diesel::sql_types::{Nullable, Text};
use diesel::*;
use helpers::*;
use schema::*;

#[test]
fn named_ref_struct() {
    #[derive(AsChangeset)]
    struct User {
        name: String,
        hair_color: String,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&User {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn named_struct() {
    #[derive(AsChangeset)]
    struct User {
        name: String,
        hair_color: String,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(User {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_explicit_table_name() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm {
        name: String,
        hair_color: String,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_path_in_table_name() {
    #[derive(AsChangeset)]
    #[table_name = "crate::schema::users"]
    struct UserForm {
        name: String,
        hair_color: String,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_lifetime() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a> {
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_multiple_lifetimes() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a, 'b> {
        name: &'a str,
        hair_color: &'b str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_lifetime_constraints() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a, 'b: 'a> {
        name: &'a str,
        hair_color: &'b str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_explicit_column_names() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a> {
        #[column_name = "name"]
        nombre: &'a str,
        #[column_name = "hair_color"]
        color_de_pelo: &'a str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            nombre: "Jim",
            color_de_pelo: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn tuple_struct() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a>(
        #[column_name = "name"] &'a str,
        #[column_name = "hair_color"] &'a str,
    );

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm("Jim", "blue"))
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn struct_containing_single_field() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a> {
        name: &'a str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm { name: "Jim" })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("black"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn tuple_struct_containing_single_field() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a>(#[column_name = "name"] &'a str);

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm("Jim"))
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("black"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_not_updated() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a> {
        #[allow(dead_code)]
        id: i32,
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            id: 3,
            name: "Jim",
            hair_color: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_based_on_column_name() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a> {
        #[column_name = "id"]
        _id: i32,
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            _id: 3,
            name: "Jim",
            hair_color: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_not_updated_with_custom_pk() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    #[primary_key(name)]
    struct UserForm<'a> {
        #[allow(dead_code)]
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Sean"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_not_updated_with_custom_composite_pk() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    #[primary_key(id, name)]
    #[allow(dead_code)]
    struct UserForm<'a> {
        id: i32,
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            id: 3,
            name: "Jim",
            hair_color: "blue",
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Sean"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn option_fields_are_skipped() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm<'a> {
        name: &'a str,
        hair_color: Option<&'a str>,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: Some("blue"),
        })
        .execute(&connection)
        .unwrap();
    update(users::table.find(2))
        .set(&UserForm {
            name: "Ruby",
            hair_color: None,
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Ruby"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn option_fields_are_assigned_null_when_specified() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    #[changeset_options(treat_none_as_null = "true")]
    struct UserForm<'a> {
        name: &'a str,
        hair_color: Option<&'a str>,
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: Some("blue"),
        })
        .execute(&connection)
        .unwrap();
    update(users::table.find(2))
        .set(&UserForm {
            name: "Ruby",
            hair_color: None,
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Ruby"), None),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn update_user_with_embed() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm {
        name: Option<String>,
        #[diesel(embed)]
        hair_color: HairColor,
    }

    #[allow(dead_code)]
    enum HairColor {
        Bald,
        Black,
        White,
        Other(String),
    }

    impl AsChangeset for HairColor {
        type Target = users::table;
        type Changeset =
            Option<Grouped<operators::Eq<users::hair_color, Bound<Nullable<Text>, String>>>>;

        fn as_changeset(self) -> Self::Changeset {
            match self {
                HairColor::Black => Some(users::hair_color.eq("Black".to_string())),
                HairColor::White => Some(users::hair_color.eq("White".to_string())),
                HairColor::Other(x) => Some(users::hair_color.eq(x)),
                HairColor::Bald => None,
            }
        }
    }

    impl AsChangeset for &HairColor {
        type Target = users::table;
        type Changeset =
            Option<Grouped<operators::Eq<users::hair_color, Bound<Nullable<Text>, String>>>>;

        fn as_changeset(self) -> Self::Changeset {
            match self {
                HairColor::Black => Some(users::hair_color.eq("Black".to_string())),
                HairColor::White => Some(users::hair_color.eq("White".to_string())),
                HairColor::Other(x) => Some(users::hair_color.eq(x.clone())),
                HairColor::Bald => None,
            }
        }
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    diesel::update(users::table)
        .filter(users::id.eq(1))
        .set(UserForm {
            name: None,
            hair_color: HairColor::Black,
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Sean"), Some(String::from("Black"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn update_user_with_embed_that_sets_null() {
    #[derive(AsChangeset)]
    #[table_name = "users"]
    struct UserForm {
        name: Option<String>,
        #[diesel(embed)]
        hair_color: HairColor,
    }

    #[allow(dead_code)]
    enum HairColor {
        Bald,
        Black,
        White,
        Other(String),
    }

    impl AsChangeset for HairColor {
        type Target = users::table;
        type Changeset =
            Option<Grouped<operators::Eq<users::hair_color, Bound<Nullable<Text>, String>>>>;

        fn as_changeset(self) -> Self::Changeset {
            match self {
                HairColor::Black => Some(users::hair_color.eq("Black".to_string())),
                HairColor::White => Some(users::hair_color.eq("White".to_string())),
                HairColor::Other(x) => Some(users::hair_color.eq(x)),
                HairColor::Bald => None,
            }
        }
    }

    impl AsChangeset for &HairColor {
        type Target = users::table;
        type Changeset =
            Option<Grouped<operators::Eq<users::hair_color, Bound<Nullable<Text>, String>>>>;

        fn as_changeset(self) -> Self::Changeset {
            match self {
                HairColor::Black => Some(users::hair_color.eq("Black".to_string())),
                HairColor::White => Some(users::hair_color.eq("White".to_string())),
                HairColor::Other(x) => Some(users::hair_color.eq(x.clone())),
                HairColor::Bald => None,
            }
        }
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    diesel::update(users::table)
        .filter(users::id.eq(1))
        .set(UserForm {
            name: None,
            hair_color: HairColor::Bald,
        })
        .execute(&connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Sean"), Some(String::from("Black"))),
        (2, String::from("Tess"), None),
    ];
    let actual = users::table.order(users::id).load(&connection);
    assert_eq!(Ok(expected), actual);
}
