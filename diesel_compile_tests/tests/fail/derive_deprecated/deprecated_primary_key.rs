#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[primary_key(id, bar = "baz")]
//~^ ERROR: expected `,`
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[primary_key(id, qux(id))]
//~^ ERROR: expected `,`
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[primary_key]
//~^ ERROR: unexpected end of input, expected parentheses
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[primary_key = id]
//~^ ERROR: attribute value must be a literal
//~| ERROR: expected parentheses
struct UserForm4 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
//~^ ERROR: Deriving `AsChangeset` on a structure that only contains primary keys isn't supported.
#[diesel(table_name = users)]
#[primary_key(id, name)]
struct UserForm5 {
    id: i32,
    name: String,
}

fn main() {}
