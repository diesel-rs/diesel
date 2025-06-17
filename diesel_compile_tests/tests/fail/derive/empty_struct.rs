#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
    }
}

#[derive(AsChangeset)]
//~^ ERROR: This derive can only be used on non-unit structs
#[diesel(table_name = users)]
struct User1;

#[derive(Identifiable)]
//~^ ERROR: This derive can only be used on non-unit structs
#[diesel(table_name = users)]
struct User2;

#[derive(Insertable)]
//~^ ERROR: This derive can only be used on non-unit structs
#[diesel(table_name = users)]
struct User3;

#[derive(Queryable)]
//~^ ERROR: This derive can only be used on non-unit structs
struct User4;

#[derive(QueryableByName)]
//~^ ERROR: This derive can only be used on non-unit structs
struct User5;

#[derive(Selectable)]
//~^ ERROR: This derive can only be used on non-unit structs
struct User6;

#[derive(Associations)]
//~^ ERROR: This derive can only be used on non-unit structs
#[diesel(table_name = users)]
struct User7;

fn main() {}
