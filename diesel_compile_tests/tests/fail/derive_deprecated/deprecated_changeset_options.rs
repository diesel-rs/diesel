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
#[changeset_options(treat_none_as_null = "true")]
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options]
//~^ ERROR: unexpected end of input, expected parentheses
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options()]
//~^ ERROR: unexpected end of input, expected identifier
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options(what)]
//~^ ERROR: expected `treat_none_as_null`
struct UserForm4 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options(treat_none_as_null)]
//~^ ERROR: unexpected end of input, expected `=`
struct UserForm5 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options(treat_none_as_null = "what")]
//~^ ERROR: expected boolean literal
struct UserForm6 {
    id: i32,
    name: String,
}

fn main() {}
