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
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_null)]
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_null = "foo")]
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
}

fn main() {}
