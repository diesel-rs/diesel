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
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_default_value)]
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(treat_none_as_default_value = "foo")]
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
}

fn main() {}
