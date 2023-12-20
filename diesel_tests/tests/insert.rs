use super::schema::*;
use diesel::*;

#[test]
fn insert_records() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();
    let actual_users = users.load::<User>(connection).unwrap();

    let expected_users = vec![
        User {
            id: actual_users[0].id,
            name: "Sean".to_string(),
            hair_color: Some("Black".to_string()),
        },
        User {
            id: actual_users[1].id,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    assert_eq!(expected_users, actual_users);
}

#[test]
fn insert_records_as_vec() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_users = vec![
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();
    let actual_users = users.load::<User>(connection).unwrap();

    let expected_users = vec![
        User {
            id: actual_users[0].id,
            name: "Sean".to_string(),
            hair_color: Some("Black".to_string()),
        },
        User {
            id: actual_users[1].id,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    assert_eq!(expected_users, actual_users);
}

#[test]
fn insert_records_as_static_array() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_users = [
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();
    let actual_users = users.load::<User>(connection).unwrap();

    let expected_users = vec![
        User {
            id: actual_users[0].id,
            name: "Sean".to_string(),
            hair_color: Some("Black".to_string()),
        },
        User {
            id: actual_users[1].id,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    assert_eq!(expected_users, actual_users);
}

#[test]
fn insert_records_as_static_array_ref() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_users = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();
    let actual_users = users.load::<User>(connection).unwrap();

    let expected_users = vec![
        User {
            id: actual_users[0].id,
            name: "Sean".to_string(),
            hair_color: Some("Black".to_string()),
        },
        User {
            id: actual_users[1].id,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    assert_eq!(expected_users, actual_users);
}

#[test]
fn insert_records_as_boxed_static_array() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_users = Box::new([
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ]);

    insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();
    let actual_users = users.load::<User>(connection).unwrap();

    let expected_users = vec![
        User {
            id: actual_users[0].id,
            name: "Sean".to_string(),
            hair_color: Some("Black".to_string()),
        },
        User {
            id: actual_users[1].id,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(all(feature = "sqlite", feature = "returning_clauses_for_sqlite_3_35"))]
fn insert_record_using_returning_clause() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_user = &NewUser::new("Sean", Some("Black"));

    let inserted_user = insert_into(users)
        .values(new_user)
        .get_result::<User>(connection)
        .unwrap();
    let expected_user = User {
        id: inserted_user.id,
        name: "Sean".to_string(),
        hair_color: Some("Black".to_string()),
    };

    assert_eq!(expected_user, inserted_user);
}

#[test]
#[cfg(all(feature = "sqlite", feature = "returning_clauses_for_sqlite_3_35"))]
fn insert_record_attached_database_using_returning_clause() {
    table! {
        external.external_table (id) {
            id -> Integer,
            data -> Text,
        }
    }

    #[derive(Queryable, Debug, PartialEq)]
    #[diesel(table_name = external_table)]
    struct ExternalEntity {
        id: i32,
        data: String,
    }

    let connection = &mut connection_without_transaction();

    // Create external table
    diesel::sql_query("ATTACH DATABASE ':memory:' AS external")
        .execute(connection)
        .unwrap();
    diesel::sql_query(
        r#"
        CREATE TABLE external.external_table (
            id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
            data text NOT NULL
        )
    "#,
    )
    .execute(connection)
    .unwrap();

    // Insert entity and fetch with the returning clause
    let inserted_entity = insert_into(external_table::table)
        .values(external_table::data.eq("test".to_string()))
        .get_result::<ExternalEntity>(connection)
        .unwrap();
    let expected_entity = ExternalEntity {
        id: inserted_entity.id,
        data: "test".to_string(),
    };

    assert_eq!(expected_entity, inserted_entity);
}

#[test]
#[cfg(not(any(feature = "sqlite", feature = "mysql")))]
fn insert_records_using_returning_clause() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    let inserted_users = insert_into(users)
        .values(new_users)
        .get_results::<User>(connection)
        .unwrap();
    let expected_users = vec![
        User {
            id: inserted_users[0].id,
            name: "Sean".to_string(),
            hair_color: Some("Black".to_string()),
        },
        User {
            id: inserted_users[1].id,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];

    assert_eq!(expected_users, inserted_users);
}

#[test]
#[cfg(all(feature = "sqlite", feature = "returning_clauses_for_sqlite_3_35"))]
fn insert_record_with_custom_returning_clause() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    let new_users = &NewUser::new("Sean", Some("Black"));

    let inserted_user = insert_into(users)
        .values(new_users)
        .returning((name, hair_color))
        .get_result::<(String, Option<String>)>(connection)
        .unwrap();
    let expected_user = ("Sean".to_string(), Some("Black".to_string()));

    assert_eq!(expected_user, inserted_user);
}

#[test]
#[cfg(not(any(feature = "sqlite", feature = "mysql")))]
fn insert_records_with_custom_returning_clause() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];

    let inserted_users = insert_into(users)
        .values(new_users)
        .returning((name, hair_color))
        .get_results::<(String, Option<String>)>(connection)
        .unwrap();
    let expected_users = vec![
        ("Sean".to_string(), Some("Black".to_string())),
        ("Tess".to_string(), None),
    ];

    assert_eq!(expected_users, inserted_users);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn batch_insert_with_defaults() {
    use crate::schema::users::table as users;
    use crate::schema_dsl::*;

    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color").not_null().default("'Green'"),
        ),
    )
    .execute(connection)
    .unwrap();

    let new_users: &[_] = &[
        NewUser::new("Sean", Some("Black")),
        NewUser::new("Tess", None),
    ];
    insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();

    let expected_users = vec![
        User {
            id: 1,
            name: "Sean".to_string(),
            hair_color: Some("Black".to_string()),
        },
        User {
            id: 2,
            name: "Tess".to_string(),
            hair_color: Some("Green".to_string()),
        },
    ];
    let actual_users = users.load(connection).unwrap();

    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn insert_with_defaults() {
    use crate::schema::users::table as users;
    use crate::schema_dsl::*;

    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color").not_null().default("'Green'"),
        ),
    )
    .execute(connection)
    .unwrap();
    insert_into(users)
        .values(&NewUser::new("Tess", None))
        .execute(connection)
        .unwrap();

    let expected_users = vec![User {
        id: 1,
        name: "Tess".to_string(),
        hair_color: Some("Green".to_string()),
    }];
    let actual_users = users.load(connection).unwrap();

    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn insert_in_nullable_with_non_null_default() {
    use crate::schema::users::table as users;
    use crate::schema_dsl::*;

    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color").default("'Green'"),
        ),
    )
    .execute(connection)
    .unwrap();

    insert_into(users)
        .values(&DefaultColorUser::new("Wylla", None))
        .execute(connection)
        .unwrap();

    insert_into(users)
        .values(&DefaultColorUser::new("Tess", Some(None)))
        .execute(connection)
        .unwrap();

    let expected_users = vec![
        User {
            id: 1,
            name: "Wylla".to_string(),
            hair_color: Some("Green".to_string()),
        },
        User {
            id: 2,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    let actual_users = users.load(connection).unwrap();

    assert_eq!(expected_users, actual_users);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn insert_returning_count_returns_number_of_rows_inserted() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    diesel::sql_query(
        "CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR NOT NULL DEFAULT 'Green'
    )",
    )
    .execute(connection)
    .unwrap();
    let new_users: &[_] = &[
        BaldUser {
            name: "Sean".to_string(),
        },
        BaldUser {
            name: "Tess".to_string(),
        },
    ];
    let count = insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();
    let second_count = insert_into(users)
        .values(&BaldUser {
            name: "Guy".to_string(),
        })
        .execute(connection)
        .unwrap();

    assert_eq!(2, count);
    assert_eq!(1, second_count);
}

#[test]
#[cfg(not(any(feature = "mysql", feature = "sqlite")))]
fn insert_with_generated_column() {
    use crate::schema::user_with_last_names::table as users;
    #[derive(Debug, Queryable, Insertable, Selectable, Default)]
    struct UserWithLastName {
        first_name: String,
        last_name: String,
        #[diesel(skip_insertion)]
        full_name: String,
    }

    let connection = &mut connection();
    diesel::sql_query(
        "CREATE TABLE user_with_last_names (
        first_name VARCHAR NOT NULL PRIMARY KEY,
        last_name VARCHAR NOT NULL,
        full_name VARCHAR GENERATED ALWAYS AS (first_name || ' ' || last_name) STORED
    )",
    )
    .execute(connection)
    .unwrap();
    let new_users: &[_] = &[UserWithLastName {
        first_name: "Sean".to_string(),
        last_name: "Black".to_string(),
        full_name: "This field not inserted".to_string(),
    }];
    let count = insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();

    assert_eq!(1, count);

    let sean_black: UserWithLastName = users.first(connection).unwrap();

    assert_eq!("Sean Black", sean_black.full_name.as_str());
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct BaldUser {
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct BorrowedUser<'a> {
    name: &'a str,
}

#[test]
fn insert_borrowed_content() {
    use crate::schema::users::table as users;
    let connection = &mut connection();
    let new_users: &[_] = &[BorrowedUser { name: "Sean" }, BorrowedUser { name: "Tess" }];
    insert_into(users)
        .values(new_users)
        .execute(connection)
        .unwrap();

    let actual_users = users.load::<User>(connection).unwrap();
    let expected_users = vec![
        User::new(actual_users[0].id, "Sean"),
        User::new(actual_users[1].id, "Tess"),
    ];

    assert_eq!(expected_users, actual_users);
}

#[test]
fn insert_empty_slice() {
    let connection = &mut connection();

    let inserted_records = insert_into(users::table)
        .values(&Vec::<NewUser>::new())
        .execute(connection);

    assert_eq!(Ok(0), inserted_records);
}

#[test]
#[cfg(feature = "postgres")]
fn insert_empty_slice_with_returning() {
    let connection = &mut connection();

    let insert_one = insert_into(users::table)
        .values(&Vec::<NewUser>::new())
        .get_result::<User>(connection);
    let insert_all = insert_into(users::table)
        .values(&Vec::<NewUser>::new())
        .get_results::<User>(connection);

    assert_eq!(Ok(None), insert_one.optional());
    assert_eq!(Ok(vec![]), insert_all);
}

#[test]
#[cfg(any(feature = "postgres", feature = "mysql"))]
fn upsert_empty_slice() {
    let connection = &mut connection();

    let inserted_records = insert_into(users::table)
        .values(&Vec::<NewUser>::new())
        .on_conflict_do_nothing()
        .execute(connection);

    assert_eq!(Ok(0), inserted_records);
}

#[test]
#[cfg(any(
    feature = "postgres",
    all(feature = "sqlite", feature = "returning_clauses_for_sqlite_3_35")
))]
fn insert_only_default_values_with_returning() {
    use crate::schema::users::id;
    use crate::schema::users::table as users;
    use crate::schema_dsl::*;
    let connection = &mut connection();

    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null().default("'Sean'"),
            string("hair_color").not_null().default("'Green'"),
        ),
    )
    .execute(connection)
    .unwrap();
    let inserted_rows = insert_into(users)
        .default_values()
        .returning(id)
        .execute(connection);
    let expected_users = vec![User::with_hair_color(1, "Sean", "Green")];

    assert_eq!(Ok(1), inserted_rows);
    assert_eq!(Ok(expected_users), users.load(connection));
}

#[test]
fn insert_single_bare_value() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    insert_into(users)
        .values(name.eq("Sean"))
        .execute(connection)
        .unwrap();

    let expected_names = vec!["Sean".to_string()];
    let actual_names = users.select(name).load(connection);
    assert_eq!(Ok(expected_names), actual_names);
}

#[test]
fn insert_single_bare_value_reference() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    insert_into(users)
        .values(&name.eq("Sean"))
        .execute(connection)
        .unwrap();

    let expected_names = vec!["Sean".to_string()];
    let actual_names = users.select(name).load(connection);
    assert_eq!(Ok(expected_names), actual_names);
}

#[test]
fn insert_multiple_bare_values() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    let new_users = vec![name.eq("Sean"), name.eq("Tess")];

    insert_into(users)
        .values(&new_users)
        .execute(connection)
        .unwrap();

    let expected_names = vec!["Sean".to_string(), "Tess".to_string()];
    let actual_names = users.select(name).load(connection);
    assert_eq!(Ok(expected_names), actual_names);
}

#[test]
fn insert_single_tuple() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    insert_into(users)
        .values((name.eq("Sean"), hair_color.eq("Brown")))
        .execute(connection)
        .unwrap();

    let expected_data = vec![("Sean".to_string(), Some("Brown".to_string()))];
    let actual_data = users.select((name, hair_color)).load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
fn insert_single_tuple_reference() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    insert_into(users)
        .values(&(name.eq("Sean"), hair_color.eq("Brown")))
        .execute(connection)
        .unwrap();

    let expected_data = vec![("Sean".to_string(), Some("Brown".to_string()))];
    let actual_data = users.select((name, hair_color)).load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
fn insert_nested_tuples() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    insert_into(users)
        .values(&(id.eq(1), (name.eq("Sean"), hair_color.eq("Brown"))))
        .execute(connection)
        .unwrap();

    let expected_data = vec![User::with_hair_color(1, "Sean", "Brown")];
    let actual_data = users.load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
fn insert_mixed_tuple_and_insertable_struct() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    let new_user = NewUser::new("Sean", Some("Brown"));
    insert_into(users)
        .values(&(id.eq(3), new_user))
        .execute(connection)
        .unwrap();

    let expected_data = vec![User::with_hair_color(3, "Sean", "Brown")];
    let actual_data = users.load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
fn insert_multiple_tuples() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    let new_users = vec![
        (name.eq("Sean"), hair_color.eq("Brown")),
        (name.eq("Tess"), hair_color.eq("Green")),
    ];
    insert_into(users)
        .values(&new_users)
        .execute(connection)
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Brown".to_string())),
        ("Tess".to_string(), Some("Green".to_string())),
    ];
    let actual_data = users.select((name, hair_color)).load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
fn insert_optional_field_with_null() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    let new_users = vec![
        (name.eq("Sean"), hair_color.eq(Some("Brown"))),
        (name.eq("Tess"), hair_color.eq(None)),
    ];
    insert_into(users)
        .values(&new_users)
        .execute(connection)
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Brown".to_string())),
        ("Tess".to_string(), None),
    ];
    let actual_data = users.select((name, hair_color)).load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
#[cfg(not(feature = "mysql"))]
fn insert_optional_field_with_default() {
    use crate::schema::users::dsl::*;
    use crate::schema_dsl::*;
    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            string("hair_color").not_null().default("'Green'"),
        ),
    )
    .execute(connection)
    .unwrap();

    let new_users = vec![
        (name.eq("Sean"), Some(hair_color.eq("Brown"))),
        (name.eq("Tess"), None),
    ];
    insert_into(users)
        .values(&new_users)
        .execute(connection)
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Brown".to_string())),
        ("Tess".to_string(), Some("Green".to_string())),
    ];
    let actual_data = users.select((name, hair_color)).load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
#[cfg(not(feature = "mysql"))]
fn insert_all_default_fields() {
    use crate::schema::users::dsl::*;
    use crate::schema_dsl::*;
    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null().default("'Tess'"),
            string("hair_color").not_null().default("'Green'"),
        ),
    )
    .execute(connection)
    .unwrap();

    let new_users = vec![
        (Some(name.eq("Sean")), Some(hair_color.eq("Brown"))),
        (None, None),
    ];
    insert_into(users)
        .values(&new_users)
        .execute(connection)
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Brown".to_string())),
        ("Tess".to_string(), Some("Green".to_string())),
    ];
    let actual_data = users.select((name, hair_color)).load(connection);
    assert_eq!(Ok(expected_data), actual_data);
}

#[test]
#[cfg(feature = "sqlite")]
fn batch_insert_is_atomic_on_sqlite() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection();

    let new_users = vec![Some(name.eq("Sean")), None];
    let result = insert_into(users).values(&new_users).execute(connection);
    assert!(result.is_err());

    assert_eq!(Ok(0), users.count().get_result(connection));
}

// regression test for https://github.com/diesel-rs/diesel/issues/2898
#[test]
fn mixed_defaultable_insert() {
    use crate::schema::users;

    #[derive(Insertable)]
    struct User {
        name: &'static str,
    }

    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct UserHairColor {
        hair_color: &'static str,
    }

    let conn = &mut connection();

    diesel::insert_into(users::table)
        .values((
            &User { name: "Bob" },
            &Some(UserHairColor {
                hair_color: "Green",
            }),
        ))
        .execute(conn)
        .unwrap();

    let actual_data = users::table
        .select((users::name, users::hair_color))
        .load(conn);

    let expected_data = vec![("Bob".to_string(), Some("Green".to_string()))];

    assert_eq!(Ok(expected_data), actual_data);
}

// regression test for https://github.com/diesel-rs/diesel/issues/3872
#[test]
fn upsert_with_composite_primary_key_do_nothing() {
    table! {
        users (id, name) {
            id -> Integer,
            name -> Text,
            hair_color -> Nullable<Text>,
        }
    }

    let conn = &mut connection_with_sean_and_tess_in_users_table();

    diesel::insert_into(users::table)
        .values((users::id.eq(1), users::name.eq("John")))
        .on_conflict_do_nothing()
        .execute(conn)
        .unwrap();
    let users = users::table
        .select(users::name)
        .load::<String>(conn)
        .unwrap();

    assert_eq!(users[0], "Sean");
    assert_eq!(users[1], "Tess");
}

// regression test for https://github.com/diesel-rs/diesel/issues/3872
#[test]
fn upsert_with_composite_primary_key_do_update() {
    table! {
        users (id, name) {
            id -> Integer,
            name -> Text,
            hair_color -> Nullable<Text>,
        }
    }

    let conn = &mut connection_with_sean_and_tess_in_users_table();

    #[cfg(feature = "mysql")]
    diesel::insert_into(users::table)
        .values((users::id.eq(1), users::name.eq("John")))
        .on_conflict(diesel::dsl::DuplicatedKeys)
        .do_update()
        .set(users::name.eq("Jane"))
        .execute(conn)
        .unwrap();

    #[cfg(not(feature = "mysql"))]
    diesel::insert_into(users::table)
        .values((users::id.eq(1), users::name.eq("John")))
        .on_conflict(users::id)
        .do_update()
        .set(users::name.eq("Jane"))
        .execute(conn)
        .unwrap();
    let users = users::table
        .select(users::name)
        .order(users::id)
        .load::<String>(conn)
        .unwrap();

    assert_eq!(users[0], "Jane");
    assert_eq!(users[1], "Tess");
}
