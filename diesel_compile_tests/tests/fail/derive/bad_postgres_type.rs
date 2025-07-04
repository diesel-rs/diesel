extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[diesel(postgres_type)]
//~^ ERROR: unexpected end of input, expected parentheses
struct Type1;

#[derive(SqlType)]
#[diesel(postgres_type())]
//~^ ERROR: expected `oid` and `array_oid` attribute or `name` attribute
struct Type2;

#[derive(SqlType)]
#[diesel(postgres_type = "foo")]
//~^ ERROR: expected parentheses
struct Type3;

#[derive(SqlType)]
#[diesel(postgres_type(name))]
//~^ ERROR: unexpected end of input, expected `=`
struct Type4;

#[derive(SqlType)]
#[diesel(postgres_type(name()))]
//~^ ERROR: expected `=`
struct Type5;

#[derive(SqlType)]
#[diesel(postgres_type(name = Foo))]
//~^ ERROR: expected string literal
struct Type6;

#[derive(SqlType)]
#[diesel(postgres_type(name = "foo", oid = 2, array_oid = 3))]
//~^ ERROR: unexpected `oid` when `name` is present
struct Type7;

#[derive(SqlType)]
#[diesel(postgres_type(name = "foo", array_oid = 3))]
//~^ ERROR: unexpected `array_oid` when `name` is present
struct Type8;

#[derive(SqlType)]
#[diesel(postgres_type(oid = 2))]
//~^ ERROR: expected `oid` and `array_oid` attribute or `name` attribute
struct Type9;

#[derive(SqlType)]
#[diesel(postgres_type(oid = 1, array_oid = "1"))]
//~^ ERROR: expected integer literal
struct Type10;

#[derive(SqlType)]
#[diesel(postgres_type(oid = "1", array_oid = 1))]
//~^ ERROR: expected integer literal
struct Type11;

#[derive(SqlType)]
#[diesel(postgres_type(schema = "foo"))]
//~^ ERROR: expected `name` to be also present
struct Type12;

#[derive(SqlType)]
#[diesel(postgres_type(what))]
//~^ ERROR: unknown attribute, expected one of `oid`, `array_oid`, `name`, `schema`
struct Type13;

#[derive(SqlType)]
#[diesel(postgres_type(schema))]
//~^ ERROR: unexpected end of input, expected `=`
struct Type14;

#[derive(SqlType)]
#[diesel(postgres_type(oid))]
//~^ ERROR: unexpected end of input, expected `=`
struct Type15;

#[derive(SqlType)]
#[diesel(postgres_type(array_oid))]
//~^ ERROR: unexpected end of input, expected `=`
struct Type16;

fn main() {}
