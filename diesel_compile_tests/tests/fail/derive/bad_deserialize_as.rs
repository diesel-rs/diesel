#[macro_use]
extern crate diesel;

#[derive(Queryable)]
struct User1 {
    id: i32,
    #[diesel(deserialize_as)]
    name: String,
}

#[derive(Queryable)]
struct User2 {
    id: i32,
    #[diesel(deserialize_as(Foo))]
    name: String,
}

#[derive(Queryable)]
struct User3 {
    id: i32,
    #[diesel(deserialize_as = "foo")]
    name: String,
}

#[derive(Queryable)]
struct User4 {
    id: i32,
    #[diesel(deserialize_as = 1omg)]
    name: String,
}

fn main() {}
