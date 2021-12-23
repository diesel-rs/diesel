use crate::schema::*;
use diesel::*;

#[test]
fn test_updating_single_column() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    update(users)
        .set(name.eq("Jim"))
        .execute(connection)
        .unwrap();

    let expected_data = vec!["Jim".to_string(); 2];
    let data: Vec<String> = users.select(name).load(connection).unwrap();
    assert_eq!(expected_data, data);
}

#[test]
fn test_updating_single_column_of_single_row() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);

    update(users.filter(id.eq(sean.id)))
        .set(name.eq("Jim"))
        .execute(connection)
        .unwrap();

    let expected_data = vec!["Jim".to_string(), "Tess".to_string()];
    let data: Vec<String> = users.select(name).order(id).load(connection).unwrap();
    assert_eq!(expected_data, data);
}

#[test]
fn test_updating_nullable_column() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);

    update(users.filter(id.eq(sean.id)))
        .set(hair_color.eq(Some("black")))
        .execute(connection)
        .unwrap();

    let data: Option<String> = users
        .select(hair_color)
        .filter(id.eq(sean.id))
        .first(connection)
        .unwrap();
    assert_eq!(Some("black".to_string()), data);

    update(users.filter(id.eq(sean.id)))
        .set(hair_color.eq(None::<String>))
        .execute(connection)
        .unwrap();

    let data: QueryResult<Option<String>> = users
        .select(hair_color)
        .filter(id.eq(sean.id))
        .first(connection);
    assert_eq!(Ok(None::<String>), data);
}

#[test]
fn test_updating_multiple_columns() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);

    update(users.filter(id.eq(sean.id)))
        .set((name.eq("Jim"), hair_color.eq(Some("black"))))
        .execute(connection)
        .unwrap();

    let expected_user = User::with_hair_color(sean.id, "Jim", "black");
    let user = users.find(sean.id).first(connection);
    assert_eq!(Ok(expected_user), user);
}

#[test]
#[cfg(not(any(feature = "sqlite", feature = "mysql")))]
fn update_returning_struct() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    let user = update(users.filter(id.eq(sean.id)))
        .set(hair_color.eq("black"))
        .get_result(connection);
    let expected_user = User::with_hair_color(sean.id, "Sean", "black");

    assert_eq!(Ok(expected_user), user);
}

#[test]
#[cfg(not(any(feature = "sqlite", feature = "mysql")))]
fn update_with_custom_returning_clause() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    let user = update(users.filter(id.eq(sean.id)))
        .set(hair_color.eq("black"))
        .returning((name, hair_color))
        .get_result::<(String, Option<String>)>(connection);
    let expected_result = ("Sean".to_string(), Some("black".to_string()));

    assert_eq!(Ok(expected_result), user);
}

#[test]
fn update_with_struct_as_changes() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    let changes = NewUser::new("Jim", Some("blue"));

    update(users.filter(id.eq(sean.id)))
        .set(&changes)
        .execute(connection)
        .unwrap();
    let user = users.find(sean.id).first(connection);
    let expected_user = User::with_hair_color(sean.id, "Jim", "blue");

    assert_eq!(Ok(expected_user), user);
}

#[test]
fn save_on_struct_with_primary_key_changes_that_struct() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    let user = User::with_hair_color(sean.id, "Jim", "blue").save_changes::<User>(connection);

    let user_in_db = users.find(sean.id).first(connection);

    assert_eq!(user, user_in_db);
}

#[test]
fn sql_syntax_is_correct_when_option_field_comes_before_non_option() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct Changes {
        hair_color: Option<String>,
        name: String,
    }

    let changes = Changes {
        hair_color: None,
        name: "Jim".into(),
    };
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    update(users::table.filter(users::id.eq(sean.id)))
        .set(&changes)
        .execute(connection)
        .unwrap();
    let user = users::table.find(sean.id).first(connection);

    let expected_user = User::new(sean.id, "Jim");
    assert_eq!(Ok(expected_user), user);
}

#[test]
fn sql_syntax_is_correct_when_option_field_comes_mixed_with_non_option() {
    #[derive(AsChangeset)]
    #[diesel(table_name = posts)]
    struct Changes {
        user_id: i32,
        title: Option<String>,
        body: String,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    let new_post = sean.new_post("Hello", Some("world"));
    insert_into(posts::table)
        .values(&new_post)
        .execute(connection)
        .unwrap();

    let changes = Changes {
        user_id: 1,
        title: None,
        body: "earth".into(),
    };
    update(posts::table)
        .set(&changes)
        .execute(connection)
        .unwrap();
    let post = posts::table
        .order(posts::id.desc())
        .first::<Post>(connection)
        .unwrap();

    let expected_post = Post::new(post.id, sean.id, "Hello".into(), Some("earth".into()));
    assert_eq!(expected_post, post);
}

#[test]
#[should_panic(expected = "There are no changes to save.")]
fn update_with_no_changes() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct Changes {
        name: Option<String>,
        hair_color: Option<String>,
    }

    let connection = &mut connection();
    let changes = Changes {
        name: None,
        hair_color: None,
    };
    update(users::table)
        .set(&changes)
        .execute(connection)
        .unwrap();
}

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn upsert_with_no_changes_executes_do_nothing() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct Changes {
        hair_color: Option<String>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let result = insert_into(users::table)
        .values(&User::new(1, "Sean"))
        .on_conflict(users::id)
        .do_update()
        .set(&Changes { hair_color: None })
        .execute(connection);

    assert_eq!(Ok(0), result);
}

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn upsert_with_no_changes_executes_do_nothing_owned() {
    #[derive(AsChangeset)]
    #[diesel(table_name = users)]
    struct Changes {
        hair_color: Option<String>,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let result = insert_into(users::table)
        .values(User::new(1, "Sean"))
        .on_conflict(users::id)
        .do_update()
        .set(&Changes { hair_color: None })
        .execute(connection);

    assert_eq!(Ok(0), result);
}

#[test]
#[cfg(feature = "postgres")]
fn upsert_with_sql_literal_for_target() {
    use crate::schema::users::dsl::*;
    use diesel::dsl::sql;
    use diesel::sql_types::Text;
    use diesel::upsert::*;

    let connection = &mut connection();
    // This index needs to happen before the insert or we'll get a deadlock
    // with any transactions that are trying to get the row lock from insert
    connection
        .execute("CREATE UNIQUE INDEX ON users (name) WHERE name != 'Tess'")
        .unwrap();
    insert_sean_and_tess_into_users_table(connection);

    let new_users = vec![
        NewUser::new("Sean", Some("Green")),
        NewUser::new("Tess", Some("Blue")),
    ];
    insert_into(users)
        .values(&new_users)
        .on_conflict(sql::<Text>("(name) WHERE name != 'Tess'"))
        .do_update()
        .set(hair_color.eq(excluded(hair_color)))
        .execute(connection)
        .unwrap();

    let data = users.select((name, hair_color)).order(id).load(connection);
    let expected_data = vec![
        ("Sean".to_string(), Some("Green".to_string())),
        ("Tess".to_string(), None),
        ("Tess".to_string(), Some("Blue".to_string())),
    ];
    assert_eq!(Ok(expected_data), data);
}
