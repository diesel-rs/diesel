#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Queryable)]
#[diesel(table_name)]
struct User1 {
    name: String,
}

#[derive(Queryable)]
#[diesel(table_name(users))]
struct User2 {
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = true)]
struct User3 {
    id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = "not a path")]
struct User4 {
    id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = does::not::exist)]
struct User5 {
    id: i32,
}

fn main() {}
