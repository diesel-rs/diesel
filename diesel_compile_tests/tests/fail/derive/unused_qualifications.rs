//@check-pass
#![deny(unused_qualifications)]
#![deny(warnings)]

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

#[derive(Queryable, QueryableByName, Selectable, Insertable, AsChangeset)]
pub struct User {
    id: i32,
    name: String,
}

#[derive(Identifiable, Associations)]
#[diesel(belongs_to(User))]
pub struct Post {
    id: i32,
    user_id: i32,
}

#[derive(diesel::MultiConnection)]
enum DbConnection {
    Pg(PgConnection),
    Sqlite(SqliteConnection),
}

#[diesel::dsl::auto_type]
fn _test() -> _ {
    users::table.select(users::id)
}
