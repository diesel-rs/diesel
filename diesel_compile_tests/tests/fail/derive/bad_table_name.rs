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
//~^ ERROR: unexpected end of input, expected `=`
struct User1 {
    name: String,
}

#[derive(Queryable)]
#[diesel(table_name(users))]
//~^ ERROR: expected `=`
struct User2 {
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = true)]
//~^ ERROR: expected identifier, found keyword `true`
struct User3 {
    id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = "not a path")]
//~^ ERROR:  expected identifier
struct User4 {
    id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = does::not::exist)]
//~^ ERROR: failed to resolve: use of unresolved module or unlinked crate `does`
struct User5 {
    id: i32,
}

fn main() {}
