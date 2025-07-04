extern crate diesel;

use diesel::alias;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);
allow_columns_to_appear_in_same_group_by_clause!(users::id, posts::id);

fn main() {
    let conn = &mut PgConnection::establish("..").unwrap();

    // this fails because `posts` is not part of the from clause
    users::table
        .group_by(posts::id)
        //~^ ERROR: type mismatch resolving `<FromClause<table> as AppearsInFromClause<table>>::Count == Once`
        .select(users::id)
        //~^ ERROR: type mismatch resolving `<id as IsContainedInGroupBy<id>>::Output == Yes`
        .execute(conn)
        .unwrap();

    // order of select and group by does not matter for the error
    users::table
        .select(users::id)
        .group_by(posts::id)
        //~^ ERROR: type mismatch resolving `<FromClause<table> as AppearsInFromClause<table>>::Count == Once`
        .execute(conn)
        .unwrap();

    let (user_alias, post_alias) = alias!(users as user1, posts as post1,);

    // this also fails if we use aliases
    user_alias
        .group_by(posts::id)
        //~^ ERROR: type mismatch resolving `<FromClause<Alias<user1>> as AppearsInFromClause<table>>::Count == Once`
        .select(user_alias.field(users::id))
        //~^ ERROR: the trait bound `AliasedField<user1, users::columns::id>: ValidGrouping<posts::columns::id>` is not satisfied
        .execute(conn)
        .unwrap();

    users::table
        .group_by(post_alias.field(posts::id))
        //~^ ERROR: type mismatch resolving `<FromClause<table> as AppearsInFromClause<Alias<post1>>>::Count == Once`
        .select(users::id)
        //~^ ERROR: the trait bound `AliasedField<post1, posts::columns::id>: IsContainedInGroupBy<users::columns::id>` is not satisfied
        .execute(conn)
        .unwrap();

    user_alias
        .group_by(post_alias.field(posts::id))
        //~^ ERROR: type mismatch resolving `<FromClause<Alias<user1>> as AppearsInFromClause<Alias<post1>>>::Count == Once`
        .select(user_alias.field(users::id))
        //~^ ERROR: the trait bound `AliasedField<user1, id>: ValidGrouping<AliasedField<post1, id>>` is not satisfied
        .execute(conn)
        .unwrap();

    user_alias
        .select(user_alias.field(users::id))
        .group_by(posts::id)
        //~^ ERROR: type mismatch resolving `<FromClause<Alias<user1>> as AppearsInFromClause<table>>::Count == Once`
        .execute(conn)
        .unwrap();

    users::table
        .select(users::id)
        .group_by(post_alias.field(posts::id))
        //~^ ERROR: type mismatch resolving `<FromClause<table> as AppearsInFromClause<Alias<post1>>>::Count == Once`
        .execute(conn)
        .unwrap();

    user_alias
        .select(user_alias.field(users::id))
        .group_by(post_alias.field(posts::id))
        //~^ ERROR: type mismatch resolving `<FromClause<Alias<user1>> as AppearsInFromClause<Alias<post1>>>::Count == Once`
        .execute(conn)
        .unwrap();
}
