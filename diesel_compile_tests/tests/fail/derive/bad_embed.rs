#[macro_use]
extern crate diesel;

table! {
    users (id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Insertable)]
struct User {
    id: i32,
    #[diesel(embed = true)]
    //~^ ERROR: expected `,`
    name: String,
}

fn main() {}
