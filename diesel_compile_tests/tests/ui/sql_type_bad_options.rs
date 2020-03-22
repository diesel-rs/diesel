extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[postgres]
struct Type1;

#[derive(SqlType)]
#[postgres(type_name = "foo", oid = "2", array_oid = "3")]
struct Type2;

#[derive(SqlType)]
#[postgres(oid = "2")]
struct Type3;

#[derive(SqlType)]
#[postgres(oid = "NaN", array_oid = "1")]
struct Type4;

#[derive(SqlType)]
#[postgres(oid = "NaN", ary_oid = "1")]
struct Type5;

#[derive(SqlType)]
#[postgres = "foo"]
struct Type6;

fn main() {}
