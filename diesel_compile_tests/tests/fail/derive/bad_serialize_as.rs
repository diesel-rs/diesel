#[macro_use]
extern crate diesel;

table! {
    users (id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User1 {
    id: i32,
    #[diesel(serialize_as)]
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User2 {
    id: i32,
    #[diesel(serialize_as(Foo))]
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User3 {
    id: i32,
    #[diesel(serialize_as = "foo")]
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User4 {
    id: i32,
    #[diesel(serialize_as = 1omg)]
    name: String,
}

fn main() {}
