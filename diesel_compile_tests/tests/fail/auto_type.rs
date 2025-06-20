extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
    }
}

joinable!(posts -> users (user_id));

#[dsl::auto_type]
fn user_has_post_with_id_greater_than(id_greater_than: i32) -> _ {
    dsl::exists(
        posts::table
            .filter(posts::user_id.eq(users::id))
            .filter(posts::id.gt(id_greater_than)),
    )
}

#[dsl::auto_type]
fn users_with_posts_with_id_greater_than(id_greater_than: i32) -> _ {
    // This fails because the macro infers the type of
    // `user_has_post_with_id_greater_than(id_greater_than)` to be
    // `user_has_post_with_id_greater_than<i32>`
    users::table
        .filter(user_has_post_with_id_greater_than(id_greater_than))
        //~^ ERROR: type alias takes 0 generic arguments but 1 generic argument was supplied
        .select(users::name)
}

#[dsl::auto_type]
fn user_has_post_with_id_greater_than_2() -> _ {
    // We check here that only the error on n shows, and that it shows only once
    // (Literals must have type suffix for auto_type, e.g. 2i64), and that it's properly spanned
    // (on the `2`)
    let n = 2;
    //~^ ERROR: literals must have type suffix for auto_type, e.g. `2_i64`
    //~| ERROR: the placeholder `_` is not allowed within types on item signatures for type aliases
    let m = 3;
    dsl::exists(
        posts::table
            .filter(posts::user_id.eq(users::id))
            .filter(posts::id.gt(n).or(posts::id.gt(n))),
    )
}

#[dsl::auto_type]
fn less_arguments_than_generics() -> _ {
    posts::user_id.eq::<_>()
    //~^ ERROR: can't infer generic argument because there is no function argument to infer from (less function arguments than generic arguments)
    //~| ERROR: the placeholder `_` is not allowed within types on item signatures for type aliases
    //~| ERROR: this method takes 1 argument but 0 arguments were supplied
}

#[derive(Queryable, Selectable)]
struct User {
    id: i32,
    name: String,
    #[diesel(select_expression = dsl::exists(
        posts::table
            .filter(posts::user_id.eq(users::id))
            .filter(posts::id.gt(2)),
        //~^ ERROR: literals must have type suffix for auto_type, e.g. `2_i64`
        //~| ERROR: the placeholder `_` is not allowed within types on item signatures for associated types
    ))]
    user_has_post_with_id_greater_than_2: bool,
}

fn main() {
    let mut conn = &mut PgConnection::establish("connection-url").unwrap();

    users_with_posts_with_id_greater_than(2).load::<String>(conn)?;
}
