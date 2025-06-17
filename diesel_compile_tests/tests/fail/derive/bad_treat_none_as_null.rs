#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_null("true"))]
//~^ ERROR: expected `=`
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_null)]
//~^ ERROR: unexpected end of input, expected `=`
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_null = "foo")]
//~^ ERROR: expected boolean literal
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserForm4 {
    id: i32,
    #[diesel(treat_none_as_null = true)]
    name: String,
    //~^ ERROR: expected `treat_none_as_null` field to be of type `Option<_>`
}

fn main() {}
