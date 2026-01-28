extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[sqlite_type]
//~^ ERROR: unexpected end of input, expected `=`
struct Type1;

#[derive(SqlType)]
#[sqlite_type()]
//~^ ERROR: expected `=`
struct Type2;

#[derive(SqlType)]
#[sqlite_type = 1]
//~^ ERROR: expected string literal
struct Type3;

#[derive(SqlType)]
//~^ ERROR: no variant or associated item named `foo` found for enum `SqliteType` in the current scope
#[sqlite_type = "foo"]
struct Type4;

fn main() {}
