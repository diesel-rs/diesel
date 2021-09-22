extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[mysql_type]
struct Type1;

#[derive(SqlType)]
#[mysql_type()]
struct Type2;

#[derive(SqlType)]
#[mysql_type = 1]
struct Type3;

#[derive(SqlType)]
#[mysql_type = "foo"]
struct Type4;

fn main() {}
