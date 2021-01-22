#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
    }
}

#[derive(AsChangeset)]
#[table_name = "users"]
struct UserStruct {
    name: String,
    #[column_name = "hair_color"]
    color_de_pelo: String,
}

#[derive(AsChangeset)]
#[table_name = "users"]
struct UserTuple(#[column_name = "name"] String);

fn main() {}
