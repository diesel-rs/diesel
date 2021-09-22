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
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options()]
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options(what)]
struct UserForm4 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options(treat_none_as_null)]
struct UserForm5 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[changeset_options(treat_none_as_null = "what")]
struct UserForm6 {
    id: i32,
    name: String,
}

fn main() {}
