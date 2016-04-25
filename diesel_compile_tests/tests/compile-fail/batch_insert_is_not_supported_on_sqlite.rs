#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::backend::Backend;
use diesel::sqlite::{Sqlite, SqliteQueryBuilder, SqliteConnection};
use diesel::types::{Integer, VarChar};

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

pub struct NewUser(String);

Insertable! {
    (users)
    pub struct NewUser(#[column_name(name)] String,);
}

fn main() {
    let connection = SqliteConnection::establish(":memory:").unwrap();

    let new_users = vec![
        NewUser("Hello".into()),
        NewUser("World".into()),
    ];
    insert(&new_users)
        .into(users::table)
        .execute(&connection);
    //~^ ERROR: SupportsDefaultKeyword
}
