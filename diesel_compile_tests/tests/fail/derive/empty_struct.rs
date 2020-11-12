#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct User1;

#[derive(Identifiable)]
#[diesel(table_name = users)]
struct User2;

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User3;

#[derive(Queryable)]
struct User4;

#[derive(QueryableByName)]
struct User5;

#[derive(Selectable)]
struct User6;

#[derive(Associations)]
#[diesel(table_name = users)]
struct User7;

fn main() {}
