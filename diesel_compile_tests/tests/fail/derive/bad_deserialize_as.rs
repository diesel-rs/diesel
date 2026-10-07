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
    //~^ ERROR: expected type
    name: String,
}

#[derive(Queryable)]
struct User4 {
    id: i32,
    #[diesel(deserialize_as = 1omg)]
    //~^ ERROR: expected type
    name: String,
}

#[derive(Queryable)]
struct User5 {
    id: i32,
    #[diesel(deserialize_as = &'static str)]
    //~^ ERROR: `deserialize_as` does not support reference types
    name: String,
}

#[derive(Queryable)]
struct User6 {
    id: i32,
    #[diesel(deserialize_as = [u8])]
    //~^ ERROR: `deserialize_as` does not support unsized slice types
    name: String,
}

#[derive(Queryable)]
struct User7 {
    id: i32,
    #[diesel(deserialize_as = dyn ToString)]
    //~^ ERROR: `deserialize_as` does not support trait objects
    name: String,
}

#[derive(Queryable)]
struct User8 {
    id: i32,
    #[diesel(deserialize_as = impl ToString)]
    //~^ ERROR: `deserialize_as` does not support impl trait types
    name: String,
}

#[derive(Queryable)]
struct User9 {
    id: i32,
    #[diesel(deserialize_as = _)]
    //~^ ERROR: `deserialize_as` does not support inference types
    name: String,
}

#[derive(Queryable)]
struct User10 {
    id: i32,
    #[diesel(deserialize_as = (&'static str, i32))]
    //~^ ERROR: `deserialize_as` does not support reference types
    name: String,
}

#[derive(Queryable)]
struct User11 {
    id: i32,
    #[diesel(deserialize_as = *const u8)]
    //~^ ERROR: `deserialize_as` does not support pointer types
    name: String,
}

#[derive(Queryable)]
struct User12 {
    id: i32,
    #[diesel(deserialize_as = fn() -> i32)]
    //~^ ERROR: `deserialize_as` does not support function pointer types
    name: String,
}

#[derive(Queryable)]
struct User13 {
    id: i32,
    #[diesel(deserialize_as = concat_idents!(Foo, Bar))]
    //~^ ERROR: macro invocation is not supported in `deserialize_as`
    name: String,
}

fn main() {}
