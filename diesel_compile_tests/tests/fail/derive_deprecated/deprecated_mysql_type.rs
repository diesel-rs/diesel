extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[mysql_type]
//~^ ERROR: unexpected end of input, expected `=`
struct Type1;

#[derive(SqlType)]
#[mysql_type()]
//~^ ERROR: expected `=
struct Type2;

#[derive(SqlType)]
#[mysql_type = 1]
//~^ ERROR: expected string literal
struct Type3;

#[derive(SqlType)]
//~^ ERROR: no variant or associated item named `foo` found for enum `MysqlType` in the current scope
#[mysql_type = "foo"]
struct Type4;

fn main() {}
