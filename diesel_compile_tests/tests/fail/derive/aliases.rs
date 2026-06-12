#[macro_use]
extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        author -> Integer,
        title -> Text,
    }
}

table! {
    pets {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts, pets);
joinable!(posts -> users (author));

pub fn check(conn: &mut PgConnection) {
    let user_alias = alias!(users as users2);
    let post_alias = alias!(posts as posts2);

    // wrong fields

    user_alias.field(posts::id);
    //~^ ERROR: type mismatch resolving `<id as QueryRelationField>::QueryRelation == table`

    // joining the same alias twice

    users::table
        .inner_join(post_alias)
        .inner_join(post_alias)
        //~^ ERROR: type mismatch resolving `<Once as Plus<Once>>::Output == Once`
        //~| ERROR: type mismatch resolving `<Join<..., ..., ...> as AppearsInFromClause<...>>::Count == Once`
        .select(users::id)
        //~^ ERROR: the method `select` exists for struct `SelectStatement<FromClause<JoinOn<..., ...>>>`, but its trait bounds were not satisfied
        .load::<i32>(conn)
        .unwrap();

    // Selecting the raw field on the aliased table
    user_alias.select(users::id).load::<i32>(conn).unwrap();
    //~^ ERROR: cannot select `users::columns::id` from `Alias<users2>`
    //~| ERROR: cannot select `users::columns::id` from `Alias<users2>`

    let user2_alias = alias!(users as user3);

    // don't allow joins to not joinable tables
    pets::table
        .inner_join(user_alias)
        //~^ ERROR: cannot join `pets::table` to `Alias<users2>` due to missing relation
        .select(pets::id)
        .load::<i32>(conn)
        .unwrap();

    // Check how error message looks when aliases to the same table are declared separately
    let post_alias_2 = alias!(posts as posts3);
    let posts = post_alias
        .inner_join(
            //~^ ERROR: the trait bound `Join<..., ..., ...>: AppearsInFromClause<...>` is not satisfied
            //~| ERROR: the trait bound `Join<..., ..., ...>: AppearsInFromClause<...>` is not satisfied
            post_alias_2.on(post_alias
                .field(posts::author)
                .eq(post_alias_2.field(posts::author))),
            //~^^^ ERROR: the trait bound `Alias<posts3>: AppearsInFromClause<Alias<posts2>>` is not satisfied
        )
        .select((post_alias.field(posts::id), post_alias_2.field(posts::id)))
        //~^ ERROR:  the method `select` exists for struct `SelectStatement<FromClause<JoinOn<..., ...>>>`, but its trait bounds were not satisfied
        .load::<(i32, i32)>(conn)
        .unwrap();
}

fn main() {}
