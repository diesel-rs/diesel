use diesel::prelude::*;
use diesel::insert_into;

table! {
    users {
        id -> Integer,
        name -> VarChar,
        hair_color -> Nullable<VarChar>,
    }
}

#[test]
fn simple_struct_definition() {
    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        name: String,
        hair_color: String,
    }

    let conn = connection();
    let new_user = NewUser {
        name: "Sean".into(),
        hair_color: "Black".into(),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(&conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color))
        .load::<(String, Option<String>)>(&conn);
    let expected = vec![("Sean".to_string(), Some("Black".to_string()))];
    assert_eq!(Ok(expected), saved);
}

macro_rules! test_struct_definition {
    ($test_name:ident, $($struct_def:tt)*) => {
        #[test]
        fn $test_name() {
            use diesel::prelude::*;
            #[derive(Insertable)]
            #[table_name = "users"]
            $($struct_def)*

            let conn = connection();
            let new_user = NewUser { name: "Sean".into(), hair_color: None };
            insert_into(users::table).values(&new_user).execute(&conn).unwrap();

            let saved = users::table.select((users::name, users::hair_color))
                .load::<(String, Option<String>)>(&conn);
            let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
            assert_eq!(Ok(expected), saved);
        }
    }
}

test_struct_definition! {
    struct_with_option_field,
    struct NewUser {
        name: String,
        hair_color: Option<String>,
    }
}

test_struct_definition! {
    pub_struct_definition,
    pub struct NewUser {
        name: String,
        hair_color: Option<String>,
    }
}

test_struct_definition! {
    struct_with_pub_field,
    pub struct NewUser {
        pub name: String,
        hair_color: Option<String>,
    }
}

test_struct_definition! {
    struct_with_pub_option_field,
    pub struct NewUser {
        name: String,
        pub hair_color: Option<String>,
    }
}

test_struct_definition! {
    named_struct_with_borrowed_body,
    struct NewUser<'a> {
        name: &'a str,
        hair_color: Option<&'a str>,
    }
}

test_struct_definition! {
    named_struct_without_trailing_comma,
    struct NewUser<'a> {
        name: &'a str,
        hair_color: Option<&'a str>
    }
}

#[test]
fn named_struct_with_renamed_field() {
    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        #[column_name = "name"] my_name: String,
        hair_color: String,
    }

    let conn = connection();
    let new_user = NewUser {
        my_name: "Sean".into(),
        hair_color: "Black".into(),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(&conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color))
        .load::<(String, Option<String>)>(&conn);
    let expected = vec![("Sean".to_string(), Some("Black".to_string()))];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn named_struct_with_renamed_option_field() {
    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser {
        #[column_name = "name"] my_name: String,
        #[column_name = "hair_color"] my_hair_color: Option<String>,
    }

    let conn = connection();
    let new_user = NewUser {
        my_name: "Sean".into(),
        my_hair_color: None,
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(&conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color))
        .load::<(String, Option<String>)>(&conn);
    let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn tuple_struct() {
    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser<'a>(
        #[column_name = "name"] &'a str,
        #[column_name = "hair_color"] Option<&'a str>,
    );

    let conn = connection();
    let new_user = NewUser("Sean", None);
    insert_into(users::table)
        .values(&new_user)
        .execute(&conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color))
        .load::<(String, Option<String>)>(&conn);
    let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn tuple_struct_without_trailing_comma() {
    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser<'a>(
        #[column_name = "name"] &'a str,
        #[column_name = "hair_color"] Option<&'a str>,
    );

    let conn = connection();
    let new_user = NewUser("Sean", None);
    insert_into(users::table)
        .values(&new_user)
        .execute(&conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color))
        .load::<(String, Option<String>)>(&conn);
    let expected = vec![("Sean".to_string(), Some("Green".to_string()))];
    assert_eq!(Ok(expected), saved);
}

#[test]
fn named_struct_with_unusual_reference_type() {
    #[derive(Insertable)]
    #[table_name = "users"]
    struct NewUser<'a> {
        name: &'a String,
        hair_color: Option<&'a String>,
    }

    let conn = connection();
    let sean = "Sean".to_string();
    let black = "Black".to_string();
    let new_user = NewUser {
        name: &sean,
        hair_color: Some(&black),
    };
    insert_into(users::table)
        .values(&new_user)
        .execute(&conn)
        .unwrap();

    let saved = users::table
        .select((users::name, users::hair_color))
        .load(&conn);
    let expected = vec![(sean.clone(), Some(black.clone()))];
    assert_eq!(Ok(expected), saved);
}

cfg_if! {
    if #[cfg(feature = "sqlite")] {
        fn connection() -> ::test_helpers::TestConnection {
            let conn = ::test_helpers::connection();
            conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR NOT NULL, hair_color VARCHAR DEFAULT 'Green')").unwrap();
            conn
        }
    } else if #[cfg(feature = "postgres")] {
        fn connection() -> ::test_helpers::TestConnection {
            let conn = ::test_helpers::connection();
            conn.execute("DROP TABLE IF EXISTS users").unwrap();
            conn.execute("CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR NOT NULL, hair_color VARCHAR DEFAULT 'Green')").unwrap();
            conn
        }

        #[test]
        fn insertable_with_slice_of_borrowed() {
            table! {
                posts {
                    id -> Serial,
                    tags -> Array<Text>,
                }
            }
            #[derive(Insertable)]
            #[table_name = "posts"]
            struct NewPost<'a> { tags: &'a [&'a str], }

            let conn = ::test_helpers::connection();
            conn.execute("DROP TABLE IF EXISTS posts").unwrap();
            conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
            let new_post = NewPost { tags: &["hi", "there"] };
            insert_into(posts::table).values(&new_post).execute(&conn).unwrap();

            let saved = posts::table.select(posts::tags).load::<Vec<String>>(&conn);
            let expected = vec![vec![String::from("hi"), String::from("there")]];
            assert_eq!(Ok(expected), saved);
        }
    } else if #[cfg(feature = "mysql")] {
        fn connection() -> ::test_helpers::TestConnection {
            let conn = ::test_helpers::connection_no_transaction();
            conn.execute("DROP TABLE IF EXISTS users").unwrap();
            conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTO_INCREMENT, name TEXT NOT NULL, hair_color VARCHAR(255) DEFAULT 'Green')").unwrap();
            conn.begin_test_transaction().unwrap();
            conn
        }
    } else {
        compile_error!(
            "At least one backend must be used to test this crate.\n \
             Pass argument `--features \"<backend>\"` with one or more of the following backends, \
             'mysql', 'postgres', or 'sqlite'. \n\n \
             ex. cargo test --features \"mysql postgres sqlite\"\n"
        );
    }
}
