use super::schema::*;
use diesel::*;

#[test]
fn selecting_basic_data() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![
        User {
            id: 1,
            name: "Sean".to_string(),
            hair_color: None,
        },
        User {
            id: 2,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    let actual_data: Vec<_> = users.select(User::as_select()).load(connection).unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn with_safe_select() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let select_name = users.select(UserName::as_select());
    let names: Vec<UserName> = select_name.load(connection).unwrap();

    assert_eq!(vec![UserName::new("Sean"), UserName::new("Tess")], names);
}

table! {
    select {
        id -> Integer,
        join -> Integer,
    }
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_and_tables_with_reserved_names() {
    use crate::schema_dsl::*;
    let connection = &mut connection();
    create_table(
        "select",
        (
            integer("id").primary_key().auto_increment(),
            integer("join").not_null(),
        ),
    )
    .execute(connection)
    .unwrap();
    connection
        .execute("INSERT INTO \"select\" (\"join\") VALUES (1), (2), (3)")
        .unwrap();

    #[derive(Debug, PartialEq, Queryable, Selectable)]
    #[diesel(table_name = select)]
    struct Select {
        join: i32,
    }

    let expected_data = vec![1, 2, 3]
        .into_iter()
        .map(|join| Select { join })
        .collect::<Vec<_>>();
    let actual_data: Vec<Select> = select::table
        .select(Select::as_select())
        .load(connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_with_different_definition_order() {
    use crate::schema_dsl::*;
    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("hair_color"),
            string("name").not_null(),
        ),
    )
    .execute(connection)
    .unwrap();
    let expected_user = User::with_hair_color(1, "Sean", "black");
    insert_into(users::table)
        .values(&NewUser::new("Sean", Some("black")))
        .execute(connection)
        .unwrap();
    let user_from_select = users::table.select(User::as_select()).first(connection);

    assert_eq!(Ok(&expected_user), user_from_select.as_ref());
}

#[test]
fn selection_using_subselect() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let ids: Vec<i32> = users::table.select(users::id).load(connection).unwrap();
    let query = format!(
        "INSERT INTO posts (user_id, title) VALUES ({}, 'Hello'), ({}, 'World')",
        ids[0], ids[1]
    );
    connection.execute(&query).unwrap();

    #[derive(Debug, PartialEq, Queryable, Selectable)]
    struct Post(#[diesel(column_name = title)] String);

    let users = users::table
        .filter(users::name.eq("Sean"))
        .select(users::id);
    let data = posts::table
        .select(Post::as_select())
        .filter(posts::user_id.eq_any(users))
        .load(connection)
        .unwrap();

    assert_eq!(vec![Post("Hello".to_string())], data);
}

#[test]
fn select_can_be_called_on_query_that_is_valid_subselect_but_invalid_query() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    let tess = find_user_by_name("Tess", connection);
    insert_into(posts::table)
        .values(&vec![
            tess.new_post("Tess", None),
            sean.new_post("Hi", None),
        ])
        .execute(connection)
        .unwrap();

    let invalid_query_but_valid_subselect = posts::table
        .select(posts::user_id)
        .filter(posts::title.eq(users::name));
    let users_with_post_using_name_as_title = users::table
        .select(User::as_select())
        .filter(users::id.eq_any(invalid_query_but_valid_subselect))
        .load(connection);

    assert_eq!(Ok(vec![tess]), users_with_post_using_name_as_title);
}

#[test]
fn selecting_multiple_aggregate_expressions_without_group_by() {
    use self::users::dsl::*;
    use diesel::dsl::{count_star, max, CountStar};
    use diesel::helper_types::max;

    #[derive(Queryable)]
    struct CountAndMax {
        count: i64,
        max_name: Option<String>,
    }
    impl<DB> Selectable<DB> for CountAndMax
    where
        DB: diesel::backend::Backend,
    {
        type SelectExpression = (CountStar, max<name>);

        fn construct_selection() -> Self::SelectExpression {
            (count_star(), max(name))
        }
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let CountAndMax { count, max_name } = users
        .select(CountAndMax::as_select())
        .get_result(connection)
        .unwrap();

    assert_eq!(2, count);
    assert_eq!(Some(String::from("Tess")), max_name);
}

#[test]
fn mixed_selectable_and_plain_select() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![
        (
            User {
                id: 1,
                name: "Sean".to_string(),
                hair_color: None,
            },
            "Sean".to_string(),
        ),
        (
            User {
                id: 2,
                name: "Tess".to_string(),
                hair_color: None,
            },
            "Tess".to_string(),
        ),
    ];
    let actual_data: Vec<_> = users
        .select((User::as_select(), name))
        .load(connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}
