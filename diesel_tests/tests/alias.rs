use crate::schema::*;
use diesel::prelude::*;

#[test]
fn selecting_basic_data() {
    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(connection)
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
        .order(user_alias.field(users::name))
        .load(connection)
        .unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn ops_with_aliases() {
    // This test should fail to compile if the std::ops::{Add, Sub, ...} impls are missing for AliasedField.
    let likes_alias = alias!(likes as likes_alias);
    let pokes_alias = alias!(pokes as pokes_alias);

    // Using pokes::poke_count and comment_id as they are columns of the same type
    let _unaliased = likes::table
        .inner_join(pokes::table.on(likes::user_id.eq(pokes::user_id)))
        .select(pokes::poke_count + likes::comment_id);
    let _aliased = likes_alias
        .inner_join(
            pokes_alias.on(likes_alias
                .field(likes::user_id)
                .eq(pokes_alias.field(pokes::user_id))),
        )
        .select(pokes_alias.field(pokes::poke_count) + likes_alias.field(likes::comment_id));
}

#[test]
fn select_multiple_from_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
    )
    .execute(connection)
    .unwrap();

    alias!(users as user_alias: UserAlias);
    let post_alias = alias!(posts as post_alias);

    // Having two different aliases in one query works
    post_alias
        .select(post_alias.fields((posts::id, posts::user_id, posts::title, posts::body)))
        .filter(
            post_alias.field(posts::user_id).eq_any(
                user_alias.select(user_alias.field(users::id)).filter(
                    user_alias
                        .field(users::id)
                        .eq_any(users::table.select(users::id)),
                ),
            ),
        )
        .load::<(i32, i32, String, Option<String>)>(connection)
        .unwrap();

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

    // Joining with explicit on clause works
    post_alias
        .left_join(users::table)
        .inner_join(
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

    // using multiple aliases for the same table works if they are declared in the same alias call
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

#[test]
fn find_and_first() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let alias = alias!(users as users_alias);
    assert_eq!(
        alias
            .find(1)
            .select(alias.field(users::name))
            .first::<String>(connection),
        Ok("Sean".into()),
    )
}

#[test]
fn boxed() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let (user1_alias, user2_alias, _post_alias) =
        alias!(users as user1, users as user2, posts as post1,);

    let q = posts::table
        .inner_join(user1_alias)
        .inner_join(user2_alias)
        .into_boxed();

    let res = q
        .select((posts::user_id, user1_alias.fields((users::id, users::id))))
        .load::<(i32, (i32, i32))>(connection)
        .unwrap();
    assert!(res.into_iter().all(|(a, (b, c))| a == b && a == c));
}

#[test]
fn visibility() {
    mod submodule {
        use super::*;
        alias! { users as user1: User1Alias }
    }
    let _user1 = submodule::user1;

    alias! {
        const USERS_ALIAS_2: Alias<UsersAlias2> = users as users_alias_2;
    }
}

// regression test for
// https://github.com/diesel-rs/diesel/issues/3319
#[test]
fn aliasing_with_group_by_and_primary_key() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let user_alias = alias!(users as user1);

    let res = user_alias
        .group_by(user_alias.field(users::id))
        .select(user_alias.field(users::name))
        .order_by(user_alias.field(users::id))
        .load::<String>(connection)
        .unwrap();
    assert!(res.len() == 2);
    assert_eq!(res[0], "Sean");
    assert_eq!(res[1], "Tess");
}
