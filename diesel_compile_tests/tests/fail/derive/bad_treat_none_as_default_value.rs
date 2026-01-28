#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_default_value())]
//~^ ERROR: expected `=`
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_default_value)]
//~^ ERROR: unexpected end of input, expected `=`
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_default_value = "foo")]
//~^ ERROR: expected boolean literal
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct UserForm4 {
    id: i32,
    #[diesel(treat_none_as_default_value = false)]
    name: String,
    //~^ ERROR:  expected `treat_none_as_default_value` field to be of type `Option<_>`
}

fn main() {}
