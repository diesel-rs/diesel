extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[diesel(sqlite_type)]
//~^ ERROR: unexpected end of input, expected parentheses
struct Type1;

#[derive(SqlType)]
#[diesel(sqlite_type())]
//~^ ERROR: expected attribute `name`
struct Type2;

#[derive(SqlType)]
#[diesel(sqlite_type = "foo")]
//~^ ERROR: expected parentheses
struct Type3;

#[derive(SqlType)]
#[diesel(sqlite_type(name))]
//~^ ERROR: unexpected end of input, expected `=`
struct Type4;

#[derive(SqlType)]
#[diesel(sqlite_type(name()))]
//~^ ERROR: expected `=`
struct Type5;

#[derive(SqlType)]
#[diesel(sqlite_type(name = Foo))]
//~^ ERROR: expected string literal
struct Type6;

#[derive(SqlType)]
#[diesel(sqlite_type(what))]
//~^ ERROR: unknown attribute, expected `name`
struct Type7;

fn main() {}
