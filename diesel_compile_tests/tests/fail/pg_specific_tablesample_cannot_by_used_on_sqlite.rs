extern crate diesel;

use diesel::dsl::*;
use diesel::query_builder::{TablesampleMethod, TablesampleSeed};
use diesel::sql_types::*;
use diesel::upsert::on_constraint;
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
        .tablesample(
            TablesampleMethod::System(10),
            TablesampleSeed::Repeatable(42),
        )
        .load::<i32>(&mut connection);
}
