#[macro_use]
extern crate diesel;

#[derive(Queryable)]
struct User1 {
    id: i32,
    #[diesel(deserialize_as)]
    //~^ ERROR: unexpected end of input, expected `=`
    name: String,
}

#[derive(Queryable)]
struct User2 {
    id: i32,
    #[diesel(deserialize_as(Foo))]
    //~^ ERROR: expected `=`
    name: String,
}

#[derive(Queryable)]
struct User3 {
    id: i32,
    #[diesel(deserialize_as = "foo")]
    //~^ ERROR: expected identifier
    name: String,
}

#[derive(Queryable)]
struct User4 {
    id: i32,
    #[diesel(deserialize_as = 1omg)]
    //~^ ERROR: expected identifier
    name: String,
}

fn main() {}
