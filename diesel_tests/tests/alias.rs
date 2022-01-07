use crate::schema::*;
use diesel::prelude::*;

#[test]
fn selecting_basic_data() {
    let connection = &mut connection();
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
        .load(connection)
        .unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_multiple_from_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
        )
        .unwrap();

    alias!(users as user_alias: UserAlias);
    let post_alias = alias!(posts as post_alias);

    // Having two different aliases in one query works
    let query = post_alias
        .select(post_alias.fields((posts::id, posts::user_id, posts::title, posts::body)))
        .filter(
            post_alias.field(posts::user_id).eq_any(
                user_alias.select(user_alias.field(users::id)).filter(
                    user_alias
                        .field(users::id)
                        .eq_any(users::table.select(users::id)),
                ),
            ),
        );

    // using a subquery with an alias seems to work
    post_alias
        .select(
            posts::table
                .select(posts::id)
                .filter(post_alias.field(posts::id).eq(posts::id))
                .single_value(),
        )
        .load::<Option<i32>>(connection)
        .unwrap();

    query
        .load::<(i32, i32, String, Option<String>)>(connection)
        .unwrap();

    post_alias
        .left_join(users::table)
        .inner_join(
            // allowing a plain join here
            // would require to write an
            // impl JoinTo<Alias<T1, F1>> for Alias<T2, F2> where T2: JoinTo<T2>
            // but I found no simple way to modify the `From clause there`
            user_alias.on(post_alias
                .field(posts::user_id)
                .eq(user_alias.field(users::id))),
        )
        .select((
            post_alias.field(posts::id),
            users::id.nullable(),
            user_alias.field(users::id),
        ))
        .load::<(i32, Option<i32>, i32)>(connection)
        .unwrap();

    // having the alias on the right side seems to work
    // joining the table with an alias twice, also works
    posts::table
        .inner_join(user_alias)
        .inner_join(users::table)
        .select((user_alias.field(users::name), users::name))
        .load::<(String, String)>(connection)
        .unwrap();

    // Joining alias to alias works
    post_alias
        .inner_join(user_alias)
        .select((user_alias.field(users::name), post_alias.field(posts::id)))
        .load::<(String, i32)>(connection)
        .unwrap();

    // we could also define and use mutiple aliases for the same table.
    // If you want to use them in the same query it's required to define them
    // in the same macro call
    let (user1_alias, user2_alias, _post_alias) =
        alias!(users as user1, users as user2, posts as post1,);

    posts::table
        .inner_join(user1_alias)
        .inner_join(user2_alias)
        .select(posts::id)
        .load::<i32>(connection)
        .unwrap();

    // its also possible to do a self join, multiple times
    // (that should work as long as all aliases are declared in the same alias! call)
    users::table
        .inner_join(user1_alias.on(users::id.eq(user1_alias.field(users::id))))
        .inner_join(
            user2_alias.on(user2_alias
                .field(users::id)
                .eq(user1_alias.field(users::id))),
        )
        .select((
            users::id,
            user1_alias.field(users::id),
            user2_alias.field(users::id),
        ))
        .load::<(i32, i32, i32)>(connection)
        .unwrap();
}
