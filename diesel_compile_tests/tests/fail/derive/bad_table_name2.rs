#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Insertable)]
#[diesel(table_name = does::not::exist)]
//~^ ERROR: cannot find module or crate `does` in this scope
struct User5 {
    id: i32,
}

fn main() {}
