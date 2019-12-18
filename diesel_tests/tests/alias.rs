use diesel::prelude::*;
use schema::*;

#[test]
fn selecting_basic_data() {
    use schema::users;

    let connection = connection();
    connection
        .execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), None::<String>),
        ("Tess".to_string(), None::<String>),
    ];

    let user_alias = alias!(users as user_alias);

    let actual_data = user_alias
        .select((
            user_alias.field(users::name),
            user_alias.field(users::hair_color),
        ))
        .load(&connection)
        .unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_multiple_from_join() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
        )
        .unwrap();

    let user_alias = alias!(users as user_alias);
    let post_alias = alias!(posts as post_alias);

    use diesel::query_builder::AsQuery;

    let query = post_alias.as_query().filter(
        post_alias
            .field(posts::user_id)
            .eq_any(user_alias.as_query().select(user_alias.field(users::id))),
    );

    println!("{:?}", diesel::debug_query::<diesel::pg::Pg, _>(&query));
    if true {
        panic!()
    }
    query
        .load::<(i32, i32, String, Option<String>, Vec<String>)>(&connection)
        .unwrap();

    post_alias.as_query().inner_join(user_alias);

    // let source = post_alias
    //     .as_query()
    //     .inner_join(
    //         user_alias.on(user_alias
    //             .field(users::id)
    //             .eq(post_alias.field(posts::user_id))),
    //     )
    //     .select((
    //         user_alias.field(users::name),
    //         post_alias.field(posts::title),
    //     ));

    // let expected_data = vec![
    //     ("Sean".to_string(), "Hello".to_string()),
    //     ("Tess".to_string(), "World".to_string()),
    // ];
    // let actual_data: Vec<_> = source.load(&connection).unwrap();

    // assert_eq!(expected_data, actual_data);
}
