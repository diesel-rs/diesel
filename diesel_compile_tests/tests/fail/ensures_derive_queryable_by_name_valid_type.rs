extern crate diesel;

use diesel::*;
use diesel::sql_types::*;

#[derive(QueryableByName)]
struct User {
    #[sql_type = "BigInt"]
	id: i32,
}

fn main() {
}
