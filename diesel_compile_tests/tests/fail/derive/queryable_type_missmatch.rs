extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        bio -> Nullable<Text>,
    }
}

#[derive(Queryable)]
struct User {
    id: i32,
    name: String,
    bio: Option<String>,
}

#[derive(Queryable)]
struct UserWithToFewFields {
    id: i32,
    name: String,
}

#[derive(Queryable)]
struct UserWithToManyFields {
    id: i32,
    name: String,
    bio: Option<String>,
    age: i32,
}

#[derive(Queryable)]
struct UserWrongOrder {
    name: String,
    id: i32,
    bio: Option<String>,
}

#[derive(Queryable)]
struct UserTypeMissmatch {
    id: i32,
    name: i32,
    bio: Option<String>,
}

#[derive(Queryable)]
struct UserNullableTypeMissmatch {
    id: i32,
    name: String,
    bio: Option<String>,
}

fn test(conn: &mut PgConnection) {
    // check that this works fine
    let _ = users::table.load::<User>(conn);

    let _ = users::table.load::<UserWithToFewFields>(conn);

    let _ = users::table.load::<UserWithToManyFields>(conn);

    let _ = users::table.load::<UserWrongOrder>(conn);

    let _ = users::table.load::<UserTypeMissmatch>(conn);

    let _ = users::table.load::<UserNullableTypeMissmatch>(conn);
}

fn main() {}
