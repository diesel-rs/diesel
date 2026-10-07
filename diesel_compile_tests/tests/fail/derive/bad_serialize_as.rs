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
    //~^ ERROR: unexpected end of input, expected `=`
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User2 {
    id: i32,
    #[diesel(serialize_as(Foo))]
    //~^ ERROR:  expected `=`
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User3 {
    id: i32,
    #[diesel(serialize_as = "foo")]
    //~^ ERROR: expected type
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User4 {
    id: i32,
    #[diesel(serialize_as = 1omg)]
    //~^ ERROR: expected type
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User5 {
    id: i32,
    #[diesel(serialize_as = *const u8)]
    //~^ ERROR: `serialize_as` does not support pointer types
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User6 {
    id: i32,
    #[diesel(serialize_as = fn() -> i32)]
    //~^ ERROR: `serialize_as` does not support function pointer types
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User7 {
    id: i32,
    #[diesel(serialize_as = _)]
    //~^ ERROR: `serialize_as` does not support inference types
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User8 {
    id: i32,
    #[diesel(serialize_as = &'static str)]
    //~^ ERROR: `serialize_as` does not support reference types
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User9 {
    id: i32,
    #[diesel(serialize_as = [u8])]
    //~^ ERROR: `serialize_as` does not support unsized slice types
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User10 {
    id: i32,
    #[diesel(serialize_as = dyn ToString)]
    //~^ ERROR: `serialize_as` does not support trait objects
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User11 {
    id: i32,
    #[diesel(serialize_as = impl ToString)]
    //~^ ERROR: `serialize_as` does not support impl trait types
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User12 {
    id: i32,
    #[diesel(serialize_as = concat_idents!(Foo, Bar))]
    //~^ ERROR: macro invocation is not supported in `serialize_as`
    name: String,
}
fn main() {}
