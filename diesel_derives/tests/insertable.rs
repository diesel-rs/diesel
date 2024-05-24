use crate::helpers::*;
use crate::schema::*;
use diesel::serialize::Output;
use diesel::*;

#[test]
fn simple_struct_definition() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let conn = &mut connection();
    let new_user = NewUser {
        name: "Sean".into(),
        hair_color: "Black".into(),
        r#type: "regular".into(),
    };
    insert_into(users::table)
        .values(new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Black".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn with_implicit_table_name() {
    #[derive(Insertable)]
    struct User {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let conn = &mut connection();
    let new_user = User {
        name: "Sean".into(),
        hair_color: "Black".into(),
        r#type: "regular".into(),
    };
    insert_into(users::table)
        .values(new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Black".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn with_path_in_table_name() {
    #[derive(Insertable)]
    #[diesel(table_name = crate::schema::users)]
    struct NewUser {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let conn = &mut connection();
    let new_user = NewUser {
        name: "Sean".into(),
        hair_color: "Black".into(),
        r#type: "regular".into(),
    };
    insert_into(users::table)
        .values(new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Black".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn simple_reference_definition() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let conn = &mut connection();
    let new_user = NewUser {
        name: "Sean".into(),
        hair_color: "Black".into(),
        r#type: "regular".into(),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Black".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn multiple_tables() {
    #[derive(Clone, Insertable)]
    #[diesel(table_name = users)]
    #[diesel(table_name = users_)]
    struct NewUser {
        name: String,
        hair_color: String,
        r#type: String,
    }

    let conn = &mut connection();
    let new_user = NewUser {
        name: "Sean".into(),
        hair_color: "Black".into(),
        r#type: "regular".into(),
    };
    insert_into(users::table)
        .values(new_user.clone())
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Black".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected.clone()), saved);

    insert_into(users_::table)
        .values(new_user)
        .execute(conn)
        .unwrap();

    let saved = users_::table
        .select((users_::name, users_::hair_color, users_::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    assert_eq!(Ok(expected), saved);
}

macro_rules! test_struct_definition {
    ($test_name:ident, $struct_def:item) => {
        #[test]
        fn $test_name() {
            #[derive(Insertable)]
            #[diesel(table_name = users)]
            $struct_def

            let conn = &mut connection();
            let new_user = NewUser { name: "Sean".into(), hair_color: None, r#type: Some("regular".into()) };
            insert_into(users::table).values(&new_user).execute(conn).unwrap();

            let saved = users::table.select((users::name, users::hair_color, users::r#type))
                .load::<(String, Option<String>, Option<String>)>(conn);
            let expected = vec![("Sean".to_string(), Some("Green".to_string()), Some("regular".to_string()))];
            assert_eq!(Ok(expected), saved);
        }
    }
}

test_struct_definition! {
    struct_with_option_field,
    struct NewUser {
        name: String,
        hair_color: Option<String>,
        r#type: Option<String>,
    }
}

test_struct_definition! {
    pub_struct_definition,
    pub struct NewUser {
        name: String,
        hair_color: Option<String>,
        r#type: Option<String>,
    }
}

test_struct_definition! {
    struct_with_pub_field,
    pub struct NewUser {
        pub name: String,
        hair_color: Option<String>,
        r#type: Option<String>,
    }
}

test_struct_definition! {
    struct_with_pub_option_field,
    pub struct NewUser {
        name: String,
        pub hair_color: Option<String>,
        r#type: Option<String>,
    }
}

test_struct_definition! {
    named_struct_with_borrowed_body,
    struct NewUser<'a> {
        name: &'a str,
        hair_color: Option<&'a str>,
        r#type: Option<&'a str>,
    }
}

#[test]
fn named_struct_with_renamed_field() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser {
        #[diesel(column_name = name)]
        my_name: String,
        hair_color: String,
        r#type: String,
    }

    let conn = &mut connection();
    let new_user = NewUser {
        my_name: "Sean".into(),
        hair_color: "Black".into(),
        r#type: "regular".into(),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Black".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn named_struct_with_renamed_option_field() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser {
        #[diesel(column_name = name)]
        my_name: String,
        #[diesel(column_name = hair_color)]
        my_hair_color: Option<String>,
        #[diesel(column_name = "type")]
        my_type: String,
    }

    let conn = &mut connection();
    let new_user = NewUser {
        my_name: "Sean".into(),
        my_hair_color: None,
        my_type: "regular".into(),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Green".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn named_struct_with_renamed_option_field_raw_type() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser {
        #[diesel(column_name = name)]
        my_name: String,
        #[diesel(column_name = hair_color)]
        my_hair_color: Option<String>,
        #[diesel(column_name = r#type)]
        my_type: String,
    }

    let conn = &mut connection();
    let new_user = NewUser {
        my_name: "Sean".into(),
        my_hair_color: None,
        my_type: "regular".into(),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Green".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn tuple_struct() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser<'a>(
        #[diesel(column_name = name)] &'a str,
        #[diesel(column_name = hair_color)] Option<&'a str>,
        #[diesel(column_name = "type")] Option<&'a str>,
    );

    let conn = &mut connection();
    let new_user = NewUser("Sean", None, Some("regular"));
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Green".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn tuple_struct_raw_type() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser<'a>(
        #[diesel(column_name = name)] &'a str,
        #[diesel(column_name = hair_color)] Option<&'a str>,
        #[diesel(column_name = r#type)] Option<&'a str>,
    );

    let conn = &mut connection();
    let new_user = NewUser("Sean", None, Some("regular"));
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load::<(String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        "Sean".to_string(),
        Some("Green".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn named_struct_with_unusual_reference_type() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser<'a> {
        name: &'a String,
        hair_color: Option<&'a String>,
        r#type: Option<&'a String>,
    }

    let conn = &mut connection();
    let sean = "Sean".to_string();
    let black = "Black".to_string();
    let regular = "regular".to_string();
    let new_user = NewUser {
        name: &sean,
        hair_color: Some(&black),
        r#type: Some(&regular),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color, users::r#type))
        .load(conn);
    let expected = vec![(sean.clone(), Some(black.clone()), Some(regular.clone()))];
    assert_eq!(Ok(expected), saved);
}

#[test]
#[cfg(all(feature = "postgres", not(feature = "sqlite")))]
fn insertable_with_slice_of_borrowed() {
    table! {
        posts {
            id -> Serial,
            tags -> Array<Text>,
        }
    }

    #[derive(Insertable)]
    #[diesel(table_name = posts)]
    struct NewPost<'a> {
        tags: &'a [&'a str],
    }

    let conn = &mut connection();
    sql_query("DROP TABLE IF EXISTS posts CASCADE")
        .execute(conn)
        .unwrap();
    sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)")
        .execute(conn)
        .unwrap();
    let new_post = NewPost {
        tags: &["hi", "there"],
    };
    insert_into(posts::table)
        .values(&new_post)
        .execute(conn)
        .unwrap();

    let saved = posts::table.select(posts::tags).load::<Vec<String>>(conn);
    let expected = vec![vec![String::from("hi"), String::from("there")]];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn embedded_struct() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct UserAttributes<'a> {
        name: &'a str,
        hair_color: &'a str,
        r#type: &'a str,
    }

    #[derive(Insertable)]
    struct User<'a> {
        id: i32,
        #[diesel(embed)]
        attributes: UserAttributes<'a>,
    }

    let conn = &mut connection();
    let new_user = User {
        id: 1,
        attributes: UserAttributes {
            name: "Sean",
            hair_color: "Black",
            r#type: "regular",
        },
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table.load::<(i32, String, Option<String>, Option<String>)>(conn);
    let expected = vec![(
        1,
        "Sean".to_string(),
        Some("Black".to_string()),
        Some("regular".to_string()),
    )];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn serialize_as_with_option() {
    use diesel::backend::Backend;
    use diesel::serialize::ToSql;
    use diesel::sql_types::Text;

    struct OptionalString(Option<&'static str>);

    impl From<OptionalString> for Option<&'static str> {
        fn from(s: OptionalString) -> Self {
            s.0
        }
    }

    struct OtherString(&'static str);

    impl From<Option<OtherString>> for MyString {
        fn from(value: Option<OtherString>) -> Self {
            MyString(value.unwrap().0.to_owned())
        }
    }

    #[derive(Debug, AsExpression)]
    #[diesel(sql_type = Text)]
    struct MyString(String);

    impl<DB> ToSql<Text, DB> for MyString
    where
        String: ToSql<Text, DB>,
        DB: Backend,
    {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> diesel::serialize::Result {
            <String as ToSql<Text, DB>>::to_sql(&self.0, out)
        }
    }

    #[derive(Insertable)]
    struct User {
        id: i32,
        #[diesel(serialize_as = MyString)]
        name: Option<OtherString>,
        #[diesel(serialize_as = Option<&'static str>)]
        hair_color: OptionalString,
    }

    let conn = &mut connection();
    let new_user = User {
        id: 1,
        name: Some(OtherString("Sean")),
        hair_color: OptionalString(Some("Black")),
    };
    insert_into(users::table)
        .values(new_user)
        .execute(conn)
        .unwrap();

    let saved = users::table
        .select((users::id, users::name, users::hair_color))
        .load::<(i32, String, Option<String>)>(conn);
    let expected = vec![(1, "Sean".to_string(), Some("Black".to_string()))];
    assert_eq!(Ok(expected), saved);
}
