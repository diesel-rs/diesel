extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
        hair_color -> VarChar,
    }
}

#[derive(Queryable, AsChangeset)]
#[diesel(table_name = users)]
pub struct User {
    name: String,
    hair_color: String,
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();
    let mut user = User {
        name: "Sean".to_string(),
        hair_color: "black".to_string(),
    };
    user.save_changes(&mut connection);
}
