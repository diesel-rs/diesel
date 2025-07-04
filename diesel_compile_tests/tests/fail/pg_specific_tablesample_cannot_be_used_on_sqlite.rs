extern crate diesel;

use diesel::dsl::*;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    let random_user_ids = users
        .tablesample_system(10)
        .with_seed(42.0)
        .load::<(i32, String)>(&mut connection);
    //~^ ERROR: the trait bound `Tablesample<table, SystemMethod>: LoadQuery<'_, SqliteConnection, ...>` is not satisfied
}
