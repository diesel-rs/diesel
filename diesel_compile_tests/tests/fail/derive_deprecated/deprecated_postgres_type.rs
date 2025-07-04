extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[postgres]
//~^ ERROR: unexpected end of input, expected parentheses
struct Type1;

#[derive(SqlType)]
#[postgres()]
//~^ ERROR: expected `oid` and `array_oid` attribute or `name` attribute
struct Type2;

#[derive(SqlType)]
#[postgres = "foo"]
//~^ ERROR: expected parentheses
struct Type3;

#[derive(SqlType)]
#[postgres(type_name)]
//~^ ERROR: unexpected end of input, expected `=`
struct Type4;

#[derive(SqlType)]
#[postgres(type_name())]
//~^ ERROR: expected `=`
struct Type5;

#[derive(SqlType)]
#[postgres(type_name = 1)]
//~^ ERROR: expected string literal
struct Type6;

#[derive(SqlType)]
#[postgres(type_name = "foo", oid = "2", array_oid = "3")]
//~^ ERROR: unexpected `oid` when `name` is present
struct Type7;

#[derive(SqlType)]
#[postgres(type_name = "foo", array_oid = "3")]
//~^ ERROR: unexpected `array_oid` when `name` is present
struct Type8;

#[derive(SqlType)]
#[postgres(oid = "2")]
//~^ ERROR: expected `oid` and `array_oid` attribute or `name` attribute
struct Type9;

#[derive(SqlType)]
#[postgres(oid = 1, array_oid = "1")]
//~^ ERROR: expected string literal
struct Type10;

#[derive(SqlType)]
#[postgres(oid = "1", array_oid = 1)]
//~^ ERROR: expected string literal
struct Type11;

#[derive(SqlType)]
#[postgres(oid = "1", array_oid = "1")]
struct Type12;

#[derive(SqlType)]
#[postgres(what)]
//~^ ERROR: unknown attribute, expected one of `oid`, `array_oid`, `type_name`
struct Type13;

fn main() {}
