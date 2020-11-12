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
    name: String,
}

fn main() {}
