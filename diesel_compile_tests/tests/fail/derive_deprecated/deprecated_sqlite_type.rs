extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[sqlite_type]
struct Type1;

#[derive(SqlType)]
#[sqlite_type()]
struct Type2;

#[derive(SqlType)]
#[sqlite_type = 1]
struct Type3;

#[derive(SqlType)]
#[sqlite_type = "foo"]
struct Type4;

fn main() {}
