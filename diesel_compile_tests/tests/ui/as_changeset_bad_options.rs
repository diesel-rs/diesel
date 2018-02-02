#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[table_name = "users"]
#[changeset_options(treat_none_as_null("true"))]
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name = "users"]
#[changeset_options(treat_none_as_null)]
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name = "users"]
#[changeset_options(treat_none_as_null = "foo")]
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name = "users"]
#[changeset_options()]
struct UserForm4 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name = "users"]
#[changeset_options = "treat_none_as_null"]
struct UserForm5 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name = "users"]
#[changeset_options]
struct UserForm6 {
    id: i32,
    name: String,
}

fn main() {}
