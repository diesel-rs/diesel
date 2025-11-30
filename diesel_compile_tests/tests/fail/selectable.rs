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
        title -> Text,
        user_id -> Integer,
    }
}
//~^^^^^^^ ERROR: cannot find type `titel` in module `posts`
//~| ERROR: cannot find value `titel` in module `posts`

joinable!(posts -> users(user_id));
allow_tables_to_appear_in_same_query!(users, posts);

#[derive(Selectable, Queryable)]
#[diesel(table_name = users)]
struct UserWithEmbeddedPost {
    id: i32,
    name: String,
    #[diesel(embed)]
    post: Post,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = users)]
struct UserWithOptionalPost {
    id: i32,
    name: String,
    #[diesel(embed)]
    post: Option<Post>,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = posts)]
struct Post {
    id: i32,
    title: String,
}

#[derive(Selectable)]
#[diesel(table_name = posts)]
struct PostWithWrongField {
    id: i32,
    // There is a typo here:
    titel: String,
}

#[derive(Selectable)]
// wrong table name here
#[diesel(table_name = post)]
//~^ ERROR: failed to resolve: use of unresolved module or unlinked crate `post`
struct PostWithWrongTableName {
    id: i32,
    title: String,
}

#[derive(Queryable)]
struct UserWithPostCount {
    id: i32,
    name: String,
    post_count: i64,
}

impl Selectable<diesel::pg::Pg> for UserWithPostCount {
    type SelectExpression = (users::id, users::name, diesel::dsl::count<posts::id>);

    fn construct_selection() -> Self::SelectExpression {
        (users::id, users::name, diesel::dsl::count(posts::id))
    }
}

#[derive(Queryable)]
struct UserWithoutSelectable {
    id: i32,
    name: String,
}

fn main() {
    let mut conn = PgConnection::establish("").unwrap();

    // supported queries
    //
    // plain queries
    let _ = posts::table
        .select(Post::as_select())
        .load(&mut conn)
        .unwrap();

    // boxed queries
    let _ = posts::table
        .into_boxed()
        .select(Post::as_select())
        .load(&mut conn)
        .unwrap();
    let _ = posts::table
        .select(Post::as_select())
        .into_boxed()
        .load(&mut conn)
        .unwrap();

    // mixed clauses
    let _ = posts::table
        .select((Post::as_select(), posts::title))
        .load::<(_, String)>(&mut conn)
        .unwrap();

    // This works for inner joins
    let _ = users::table
        .inner_join(posts::table)
        .select(UserWithEmbeddedPost::as_select())
        .load(&mut conn)
        .unwrap();

    // also for left joins
    let _ = users::table
        .left_join(posts::table)
        .select(UserWithOptionalPost::as_select())
        .load(&mut conn)
        .unwrap();

    // allow manual impls with complex expressions
    // (and group by)
    let _ = users::table
        .inner_join(posts::table)
        .group_by(users::id)
        .select(UserWithPostCount::as_select())
        .load(&mut conn)
        .unwrap();

    // inserts
    let _ = diesel::insert_into(posts::table)
        .values(posts::title.eq(""))
        .returning(Post::as_select())
        .load(&mut conn)
        .unwrap();

    // update
    let _ = diesel::update(posts::table)
        .set(posts::title.eq(""))
        .returning(Post::as_select())
        .load(&mut conn)
        .unwrap();

    // delete
    let _ = diesel::delete(posts::table)
        .returning(Post::as_select())
        .load(&mut conn)
        .unwrap();

    // forbidden queries
    //
    // left joins force nullable
    let _ = users::table
        .left_join(posts::table)
        .select(UserWithEmbeddedPost::as_select())
        //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
        //~| ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        .load(&mut conn)
        //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
        //~| ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        .unwrap();

    // group by clauses are considered
    let _ = users::table
        .inner_join(posts::table)
        .group_by(posts::id)
        .select(UserWithEmbeddedPost::as_select())
        //~^ ERROR: the trait bound `posts::columns::id: IsContainedInGroupBy<users::columns::id>` is not satisfied
        //~| ERROR: the trait bound `posts::columns::id: IsContainedInGroupBy<users::columns::name>` is not satisfied
        .load(&mut conn)
        .unwrap();

    // missing group by clause
    let _ = users::table
        .inner_join(posts::table)
        .select(UserWithPostCount::as_select())
        //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL
        .load(&mut conn)
        .unwrap();

    // cannot load results from more than one table via
    // returning clauses
    let _ = diesel::insert_into(users::table)
        .values(users::name.eq(""))
        .returning(UserWithEmbeddedPost::as_select())
        //~^ ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .load(&mut conn)
        //~^ ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .unwrap();

    // cannot load results from more than one table via
    // returning clauses
    let _ = diesel::update(users::table)
        .set(users::name.eq(""))
        .returning(UserWithEmbeddedPost::as_select())
        //~^ ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .load(&mut conn)
        //~^ ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .unwrap();

    // cannot load results from more than one table via
    // returning clauses
    let _ = diesel::delete(users::table)
        .returning(UserWithEmbeddedPost::as_select())
        //~^ ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .load(&mut conn)
        //~^ ERROR: cannot select `posts::columns::id` from `users::table`
        //~| ERROR: cannot select `posts::columns::title` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .unwrap();

    // cannot use this method without deriving selectable
    let _ = users::table
        .select(UserWithoutSelectable::as_select())
        //~^ ERROR: trait bounds were not satisfied
        .load(&mut conn)
        .unwrap();

    // type locking
    let _ = posts::table
        .select(Post::as_select())
        .load::<(i32, String)>(&mut conn)
        //~^ ERROR: the trait bound `diesel::expression::select_by::SelectBy<Post, _>: SingleValue` is not satisfied
        //~| ERROR: the trait bound `(i32, String): Queryable<SelectBy<Post, _>, _>` is not satisfied
        .unwrap();
    let _ = posts::table
        .select(Post::as_select())
        .into_boxed()
        .load::<(i32, String)>(&mut conn)
        //~^ ERROR: the trait bound `diesel::expression::select_by::SelectBy<Post, _>: SingleValue` is not satisfied
        //~| ERROR: the trait bound `(i32, String): Queryable<SelectBy<Post, _>, _>` is not satisfied
        .unwrap();
    let _ = posts::table
        .select((Post::as_select(), posts::title))
        .load::<((i32, String), String)>(&mut conn)
        //~^ ERROR: the trait bound `(SelectBy<Post, _>, Text): CompatibleType<((i32, String), String), _>` is not satisfied
        .unwrap();
    let _ = diesel::insert_into(posts::table)
        .values(posts::title.eq(""))
        .returning(Post::as_select())
        .load::<(i32, String, i32)>(&mut conn)
        //~^ ERROR: the trait bound `diesel::expression::select_by::SelectBy<Post, _>: SingleValue` is not satisfied
        //~| ERROR: the trait bound `(i32, String, i32): Queryable<SelectBy<Post, _>, _>` is not satisfied
        .unwrap();

    // cannot use backend specific selectable with other backend
    let mut conn = SqliteConnection::establish("").unwrap();
    let _ = users::table
        .inner_join(posts::table)
        .group_by(users::id)
        .select(UserWithPostCount::as_select())
        .load(&mut conn)
        //~^ ERROR: type mismatch resolving `<SqliteConnection as Connection>::Backend == Pg`
        .unwrap();
}
