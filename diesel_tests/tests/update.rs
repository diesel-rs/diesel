use schema::*;
use diesel::*;

#[test]
fn test_updating_single_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    update(users).set(name.eq("Jim")).execute(&connection).unwrap();

    let expected_data = vec!["Jim".to_string(); 2];
    let data: Vec<String> = users.select(name).load(&connection).unwrap();
    assert_eq!(expected_data, data);
}

#[test]
fn test_updating_single_column_of_single_row() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);

    update(users.filter(id.eq(sean.id))).set(name.eq("Jim"))
        .execute(&connection).unwrap();

    let expected_data = vec!["Jim".to_string(), "Tess".to_string()];
    let data: Vec<String> = users.select(name).order(id).load(&connection).unwrap();
    assert_eq!(expected_data, data);
}

#[test]
fn test_updating_nullable_column() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);

    update(users.filter(id.eq(sean.id))).set(hair_color.eq(Some("black")))
        .execute(&connection).unwrap();

    let data: Option<String> = users.select(hair_color)
        .filter(id.eq(sean.id))
        .first(&connection)
        .unwrap();
    assert_eq!(Some("black".to_string()), data);

    update(users.filter(id.eq(sean.id))).set(hair_color.eq(None::<String>))
        .execute(&connection).unwrap();

    let data: QueryResult<Option<String>> = users.select(hair_color)
        .filter(id.eq(sean.id))
        .first(&connection);
    assert_eq!(Ok(None::<String>), data);
}

#[test]
fn test_updating_multiple_columns() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);

    update(users.filter(id.eq(sean.id))).set((
        name.eq("Jim"),
        hair_color.eq(Some("black")),
    )).execute(&connection).unwrap();

    let expected_user = User::with_hair_color(sean.id, "Jim", "black");
    let user = users.find(sean.id).first(&connection);
    assert_eq!(Ok(expected_user), user);
}

#[test]
#[cfg(not(any(feature="sqlite", feature="mysql")))]
fn update_returning_struct() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let user = update(users.filter(id.eq(sean.id))).set(hair_color.eq("black"))
        .get_result(&connection);
    let expected_user = User::with_hair_color(sean.id, "Sean", "black");

    assert_eq!(Ok(expected_user), user);
}

#[test]
#[cfg(not(any(feature="sqlite", feature="mysql")))]
fn update_with_custom_returning_clause() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let user = update(users.filter(id.eq(sean.id)))
        .set(hair_color.eq("black"))
        .returning((name, hair_color))
        .get_result::<(String, Option<String>)>(&connection);
    let expected_result = ("Sean".to_string(), Some("black".to_string()));

    assert_eq!(Ok(expected_result), user);
}

#[test]
fn update_with_struct_as_changes() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let changes = NewUser::new("Jim", Some("blue"));

    update(users.filter(id.eq(sean.id))).set(&changes)
        .execute(&connection).unwrap();
    let user = users.find(sean.id).first(&connection);
    let expected_user = User::with_hair_color(sean.id, "Jim", "blue");

    assert_eq!(Ok(expected_user), user);
}

#[test]
fn update_with_struct_does_not_set_primary_key() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let other_id = sean.id + 1;
    let changes = User::with_hair_color(other_id, "Jim", "blue");

    update(users.filter(id.eq(sean.id))).set(&changes)
        .execute(&connection).unwrap();
    let user = users.find(sean.id).first(&connection);
    let expected_user = User::with_hair_color(sean.id, "Jim", "blue");

    assert_eq!(Ok(expected_user), user);
}

#[test]
fn save_on_struct_with_primary_key_changes_that_struct() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let user = User::with_hair_color(sean.id, "Jim", "blue").save_changes::<User>(&connection);

    let user_in_db = users.find(sean.id).first(&connection);

    assert_eq!(user, user_in_db);
}

#[test]
fn option_fields_on_structs_are_not_assigned() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    update(users.filter(id.eq(sean.id)))
        .set(hair_color.eq("black"))
        .execute(&connection).unwrap();
    let user = User::new(sean.id, "Jim").save_changes(&connection);

    let expected_user = User::with_hair_color(sean.id, "Jim", "black");
    assert_eq!(Ok(expected_user), user);
}

#[test]
fn sql_syntax_is_correct_when_option_field_comes_before_non_option() {
    #[derive(AsChangeset)]
    #[table_name="users"]
    struct Changes {
        hair_color: Option<String>,
        name: String,
    }

    let changes = Changes { hair_color: None, name: "Jim".into() };
    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    update(users::table.filter(users::id.eq(sean.id))).set(&changes)
        .execute(&connection).unwrap();
    let user = users::table.find(sean.id).first(&connection);

    let expected_user = User::new(sean.id, "Jim");
    assert_eq!(Ok(expected_user), user);
}

#[test]
fn sql_syntax_is_correct_when_option_field_comes_mixed_with_non_option() {
    #[derive(AsChangeset)]
    #[table_name="posts"]
    struct Changes {
        user_id: i32,
        title: Option<String>,
        body: String,
    }

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let new_post = sean.new_post("Hello", Some("world"));
    insert(&new_post).into(posts::table).execute(&connection).unwrap();

    let changes = Changes { user_id: 1, title: None, body: "earth".into() };
    update(posts::table)
        .set(&changes)
        .execute(&connection)
        .unwrap();
    let post = posts::table.order(posts::id.desc()).first::<Post>(&connection).unwrap();

    let expected_post = Post::new(post.id, sean.id, "Hello".into(), Some("earth".into()));
    assert_eq!(expected_post, post);
}

#[test]
fn can_update_with_struct_containing_single_field() {
    #[derive(AsChangeset)]
    #[table_name="posts"]
    struct SetBody {
        body: String,
    }

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let new_post = sean.new_post("Hello", Some("world"));
    insert(&new_post).into(posts::table).execute(&connection).unwrap();

    let changes = SetBody { body: "earth".into() };
    update(posts::table)
        .set(&changes)
        .execute(&connection)
        .unwrap();
    let post = posts::table.order(posts::id.desc()).first::<Post>(&connection).unwrap();

    let expected_post = Post::new(post.id, sean.id, "Hello".into(), Some("earth".into()));
    assert_eq!(expected_post, post);
}

#[test]
fn struct_with_option_fields_treated_as_null() {
    #[derive(Identifiable, AsChangeset)]
    #[table_name="posts"]
    #[changeset_options(treat_none_as_null="true")]
    struct UpdatePost {
        id: i32,
        title: String,
        body: Option<String>,
    }

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &connection);
    let new_post = sean.new_post("Hello", Some("world"));
    insert(&new_post).into(posts::table)
        .execute(&connection).unwrap();
    let post = posts::table.order(posts::id.desc()).first::<Post>(&connection).unwrap();

    let changes = UpdatePost { id: post.id, title: "Hello again".into(), body: None };
    let expected_post = Post::new(post.id, sean.id, "Hello again".into(), None);
    let updated_post = changes.save_changes(&connection);
    let post_in_database = posts::table.find(post.id).first(&connection);

    assert_eq!(Ok(&expected_post), updated_post.as_ref());
    assert_eq!(Ok(&expected_post), post_in_database.as_ref());
}

#[test]
#[should_panic(expected="There are no changes to save.")]
fn update_with_no_changes() {
    #[derive(AsChangeset)]
    #[table_name="users"]
    struct Changes {
        name: Option<String>,
        hair_color: Option<String>,
    }

    let connection = connection();
    let changes = Changes { name: None, hair_color: None, };
    update(users::table).set(&changes).execute(&connection).unwrap();
}

#[test]
#[cfg(feature="postgres")]
fn upsert_with_no_changes_executes_do_nothing() {
    use diesel::pg::upsert::*;

    #[derive(AsChangeset)]
    #[table_name="users"]
    struct Changes {
        hair_color: Option<String>,
    }

    let connection = connection_with_sean_and_tess_in_users_table();
    let result = insert(&User::new(1, "Sean")
       .on_conflict(users::id, do_update().set(&Changes { hair_color: None }))
    ).into(users::table).execute(&connection);

    assert_eq!(Ok(0), result);
}
