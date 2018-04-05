#[macro_use]
extern crate diesel;

use diesel::sql_types::{Nullable, Text};
use diesel::pg::Pg;

table! {
    users {
        id -> Integer,
    }
}

#[derive(Queryable)]
#[table_name = "users"]
#[check_types(backend = "Pg")]
struct User {
    id: String,
    #[sql_type = "Nullable<Text>"]
    name: String,
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
    }
}

#[derive(Queryable)]
#[table_name = "posts"]
#[check_types(backend = "Pg")]
struct Post {
    user_id: i32,
    id: i32,
}

fn main() {}
