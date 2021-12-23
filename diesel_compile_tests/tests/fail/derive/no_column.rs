#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct1 {
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct2 {
    #[diesel(column_name = name)]
    full_name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserTuple(#[diesel(column_name = name)] String);

fn main() {}
