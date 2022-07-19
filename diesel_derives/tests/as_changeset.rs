use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
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

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&User {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn named_struct() {
    #[derive(AsChangeset)]
    struct User {
        name: String,
        hair_color: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(User {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_explicit_table_name() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm {
        name: String,
        hair_color: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_path_in_table_name() {
    #[derive(AsChangeset)]
    #[diesel(table_name = crate::schema::users)]
    struct UserForm {
        name: String,
        hair_color: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_lifetime() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_multiple_lifetimes() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a, 'b> {
        name: &'a str,
        hair_color: &'b str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_lifetime_constraints() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a, 'b: 'a> {
        name: &'a str,
        hair_color: &'b str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_explicit_column_names() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        #[diesel(column_name = name)]
        nombre: &'a str,
        #[diesel(column_name = hair_color)]
        color_de_pelo: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            nombre: "Jim",
            color_de_pelo: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_serialize_as() {
    #[derive(Debug, FromSqlRow, AsExpression)]
    #[diesel(sql_type = sql_types::Text)]
    struct UppercaseString(pub String);

    impl From<String> for UppercaseString {
        fn from(val: String) -> Self {
            UppercaseString(val.to_uppercase())
        }
    }

    impl<DB> serialize::ToSql<sql_types::Text, DB> for UppercaseString
    where
        DB: backend::Backend,
        String: serialize::ToSql<sql_types::Text, DB>,
    {
        fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, DB>) -> serialize::Result {
            self.0.to_sql(out)
        }
    }

    #[derive(AsChangeset)]
    struct User {
        #[diesel(serialize_as = UppercaseString)]
        name: String,
        #[diesel(serialize_as = UppercaseString)]
        hair_color: Option<String>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(User {
            name: String::from("Jim"),
            hair_color: Some(String::from("blue")),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("JIM"), Some(String::from("BLUE"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn tuple_struct() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a>(
        #[diesel(column_name = name)] &'a str,
        #[diesel(column_name = hair_color)] &'a str,
    );

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm("Jim", "blue"))
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn struct_containing_single_field() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        name: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm { name: "Jim" })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("black"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn tuple_struct_containing_single_field() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a>(#[diesel(column_name = name)] &'a str);

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm("Jim"))
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("black"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_not_updated() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        #[allow(dead_code)]
        id: i32,
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            id: 3,
            name: "Jim",
            hair_color: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_based_on_column_name() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        #[diesel(column_name = id)]
        _id: i32,
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            _id: 3,
            name: "Jim",
            hair_color: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_not_updated_with_custom_pk() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    #[diesel(primary_key(name))]
    struct UserForm<'a> {
        #[allow(dead_code)]
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Sean"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn primary_key_is_not_updated_with_custom_composite_pk() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    #[diesel(primary_key(id, name))]
    #[allow(dead_code)]
    struct UserForm<'a> {
        id: i32,
        name: &'a str,
        hair_color: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            id: 3,
            name: "Jim",
            hair_color: "blue",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Sean"), Some(String::from("blue"))),
        (2, String::from("Tess"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn option_fields_are_skipped() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        name: &'a str,
        hair_color: Option<&'a str>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: Some("blue"),
        })
        .execute(connection)
        .unwrap();
    update(users::table.find(2))
        .set(&UserForm {
            name: "Ruby",
            hair_color: None,
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Ruby"), Some(String::from("brown"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn option_fields_are_assigned_null_when_specified() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    #[diesel(treat_none_as_null = true)]
    struct UserForm<'a> {
        name: &'a str,
        hair_color: Option<&'a str>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: Some("blue"),
        })
        .execute(connection)
        .unwrap();
    update(users::table.find(2))
        .set(&UserForm {
            name: "Ruby",
            hair_color: None,
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (1, String::from("Jim"), Some(String::from("blue"))),
        (2, String::from("Ruby"), None),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}
