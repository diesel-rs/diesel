extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[diesel(mysql_type)]
struct Type1;

#[derive(SqlType)]
#[diesel(mysql_type())]
struct Type2;

#[derive(SqlType)]
#[diesel(mysql_type = "foo")]
struct Type3;

#[derive(SqlType)]
#[diesel(mysql_type(name))]
struct Type4;

#[derive(SqlType)]
#[diesel(mysql_type(name()))]
struct Type5;

#[derive(SqlType)]
#[diesel(mysql_type(name = Foo))]
struct Type6;

#[derive(SqlType)]
#[diesel(mysql_type(what))]
struct Type7;

fn main() {}
