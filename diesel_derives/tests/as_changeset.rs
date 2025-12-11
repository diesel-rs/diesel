use crate::helpers::*;
use crate::schema::*;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
use diesel::*;

#[test]
fn named_ref_struct() {
    #[derive(AsChangeset)]
    struct User {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&User {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
            r#type: String::from("super"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(User {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
            r#type: String::from("super"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
            r#type: String::from("super"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: String::from("Jim"),
            hair_color: String::from("blue"),
            r#type: String::from("super"),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
            r#type: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_multiple_lifetimes() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a, 'b, 'c> {
        name: &'a str,
        hair_color: &'b str,
        r#type: &'c str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
            r#type: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_lifetime_constraints() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a, 'b: 'a, 'c: 'b> {
        name: &'a str,
        hair_color: &'b str,
        r#type: &'c str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
            r#type: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        #[diesel(column_name = "type")]
        tipe: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            nombre: "Jim",
            color_de_pelo: "blue",
            tipe: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn with_explicit_column_names_raw_type() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        #[diesel(column_name = name)]
        nombre: &'a str,
        #[diesel(column_name = hair_color)]
        color_de_pelo: &'a str,
        #[diesel(column_name = r#type)]
        tipe: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            nombre: "Jim",
            color_de_pelo: "blue",
            tipe: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        #[diesel(serialize_as = UppercaseString)]
        r#type: Option<String>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(User {
            name: String::from("Jim"),
            hair_color: Some(String::from("blue")),
            r#type: Some(String::from("super")),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("JIM"),
            Some(String::from("BLUE")),
            Some(String::from("SUPER")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        #[diesel(column_name = "type")] &'a str,
    );

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm("Jim", "blue", "super"))
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn tuple_struct_raw_type() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a>(
        #[diesel(column_name = name)] &'a str,
        #[diesel(column_name = hair_color)] &'a str,
        #[diesel(column_name = r#type)] &'a str,
    );

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm("Jim", "blue", "super"))
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        (
            1,
            String::from("Jim"),
            Some(String::from("black")),
            Some(String::from("regular")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        (
            1,
            String::from("Jim"),
            Some(String::from("black")),
            Some(String::from("regular")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            id: 3,
            name: "Jim",
            hair_color: "blue",
            r#type: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            _id: 3,
            name: "Jim",
            hair_color: "blue",
            r#type: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: "blue",
            r#type: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Sean"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: &'a str,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            id: 3,
            name: "Jim",
            hair_color: "blue",
            r#type: "super",
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Sean"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        r#type: Option<&'a str>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            hair_color: Some("blue"),
            r#type: Some("super"),
        })
        .execute(connection)
        .unwrap();
    update(users::table.find(2))
        .set(&UserForm {
            name: "Ruby",
            hair_color: None,
            r#type: None,
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Ruby"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
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
        #[diesel(treat_none_as_null = false)]
        name: Option<&'a str>,
        hair_color: Option<&'a str>,
        #[diesel(treat_none_as_null = false)]
        r#type: Option<&'a str>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: None,
            hair_color: Some("blue"),
            r#type: None,
        })
        .execute(connection)
        .unwrap();
    update(users::table.find(2))
        .set(&UserForm {
            name: Some("Ruby"),
            hair_color: None,
            r#type: None,
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Sean"),
            Some(String::from("blue")),
            Some(String::from("regular")),
        ),
        (2, String::from("Ruby"), None, Some(String::from("admin"))),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
#[allow(unused_parens)]
fn option_fields_are_correctly_detected() {
    diesel::table! {
        test_table (id) {
            id -> Int8,
            test -> Text,
        }
    }

    macro_rules! define {
        ($field_ty:ty) => {
            #[derive(diesel::AsChangeset)]
            #[diesel(table_name = test_table)]
            pub struct S1 {
                pub test: (($field_ty)),
            }

            #[derive(diesel::AsChangeset)]
            #[diesel(table_name = test_table)]
            pub struct S2 {
                pub test: (((Option<String>))),
            }
        };
    }

    // Causes a compile error if the field is not detected as `Option<T>`
    define!((((Option<String>))));
}

#[test]
fn embedded_struct() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserAttributes<'a> {
        hair_color: &'a str,
        r#type: &'a str,
    }

    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        name: &'a str,
        #[diesel(embed)]
        attribute: UserAttributes<'a>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            attribute: UserAttributes {
                hair_color: "blue",
                r#type: "super",
            },
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn skip_update() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    #[diesel(primary_key(name))]
    #[allow(dead_code)]
    struct UserForm<'a> {
        #[diesel(skip_update)]
        name: &'a str,
        hair_color: &'a str,
        r#type: &'a str,
        #[diesel(skip_update)]
        non_existing: i32,
    }

    diesel::update(users::table.find(1))
        .set(UserForm {
            name: "John",
            hair_color: "green",
            r#type: "admin",
            non_existing: 42,
        })
        .execute(connection)
        .unwrap();

    let res = users::table
        .find(1)
        .first::<(i32, String, Option<String>, Option<String>)>(connection)
        .unwrap();

    assert_eq!(res.1, "Sean");
    assert_eq!(res.2, Some("green".into()));
    assert_eq!(res.3, Some("admin".into()));
}

#[test]
fn optional_embedded_struct() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserAttributes<'a> {
        hair_color: &'a str,
        r#type: &'a str,
    }

    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct UserForm<'a> {
        name: &'a str,
        #[diesel(embed)]
        attribute: Option<UserAttributes<'a>>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    update(users::table.find(1))
        .set(&UserForm {
            name: "Jim",
            attribute: Some(UserAttributes {
                hair_color: "blue",
                r#type: "super",
            }),
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);

    update(users::table.find(2))
        .set(&UserForm {
            name: "Tess",
            attribute: None,
        })
        .execute(connection)
        .unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("brown")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}

#[test]
fn named_struct_batch() {
    #[derive(Debug, Clone, AsChangeset, Insertable)]
    struct User {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let jim = User {
        name: String::from("Jim"),
        hair_color: String::from("blue"),
        r#type: String::from("super"),
    };
    let tess = User {
        name: String::from("Tess"),
        hair_color: String::from("blue"),
        r#type: String::from("super"),
    };

    let mut users: Vec<User> = Vec::new();
    users.push(jim.clone());
    users.push(tess.clone());

    // Control: See if query of assign remains unchanged.
    let update_jim = update(users::table.find(1)).set(&jim);
    let debug_jim = diesel::debug_query::<crate::helpers::TestBackend, _>(&update_jim);
    println!("debug_jim\n-------------{:?}", debug_jim);

    // TODO: batch update.
    let update_users = update(users::table.find(1)).set(&users);
    let debug_users = diesel::debug_query::<crate::helpers::TestBackend, _>(&update_users);
    println!("debug_users\n-------------{:?}\n-------------", debug_users);

    // Should update both Jim and Tess
    update_users.execute(connection).unwrap();

    let expected = vec![
        (
            1,
            String::from("Jim"),
            Some(String::from("blue")),
            Some(String::from("super")),
        ),
        (
            2,
            String::from("Tess"),
            Some(String::from("blue")),
            Some(String::from("admin")),
        ),
    ];
    let actual = users::table.order(users::id).load(connection);
    assert_eq!(Ok(expected), actual);
}
