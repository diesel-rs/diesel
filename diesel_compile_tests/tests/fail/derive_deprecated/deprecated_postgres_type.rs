extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[postgres]
struct Type1;

#[derive(SqlType)]
#[postgres()]
struct Type2;

#[derive(SqlType)]
#[postgres = "foo"]
struct Type3;

#[derive(SqlType)]
#[postgres(type_name)]
struct Type4;

#[derive(SqlType)]
#[postgres(type_name())]
struct Type5;

#[derive(SqlType)]
#[postgres(type_name = 1)]
struct Type6;

#[derive(SqlType)]
#[postgres(type_name = "foo", oid = "2", array_oid = "3")]
struct Type7;

#[derive(SqlType)]
#[postgres(type_name = "foo", array_oid = "3")]
struct Type8;

#[derive(SqlType)]
#[postgres(oid = "2")]
struct Type9;

#[derive(SqlType)]
#[postgres(oid = 1, array_oid = "1")]
struct Type10;

#[derive(SqlType)]
#[postgres(oid = "1", array_oid = 1)]
struct Type11;

#[derive(SqlType)]
#[postgres(oid = "1", array_oid = "1")]
struct Type12;

#[derive(SqlType)]
#[postgres(what)]
struct Type13;

fn main() {}
