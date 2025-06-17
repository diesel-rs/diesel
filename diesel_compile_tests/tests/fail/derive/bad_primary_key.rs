#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[diesel(primary_key(id, bar = "baz"))]
//~^ ERROR: expected `,`
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(primary_key(id, qux(id)))]
//~^ ERROR: expected `,`
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(primary_key)]
//~^ ERROR:  unexpected end of input, expected parentheses
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(primary_key = id)]
//~^ ERROR: expected parentheses
struct UserForm4 {
    id: i32,
    name: String,
}

fn main() {}
