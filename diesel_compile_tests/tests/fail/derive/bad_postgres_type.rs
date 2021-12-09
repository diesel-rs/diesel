extern crate diesel;

use diesel::sql_types::SqlType;

#[derive(SqlType)]
#[diesel(postgres_type)]
struct Type1;

#[derive(SqlType)]
#[diesel(postgres_type())]
struct Type2;

#[derive(SqlType)]
#[diesel(postgres_type = "foo")]
struct Type3;

#[derive(SqlType)]
#[diesel(postgres_type(name))]
struct Type4;

#[derive(SqlType)]
#[diesel(postgres_type(name()))]
struct Type5;

#[derive(SqlType)]
#[diesel(postgres_type(name = Foo))]
struct Type6;

#[derive(SqlType)]
#[diesel(postgres_type(name = "foo", oid = 2, array_oid = 3))]
struct Type7;

#[derive(SqlType)]
#[diesel(postgres_type(name = "foo", array_oid = 3))]
struct Type8;

#[derive(SqlType)]
#[diesel(postgres_type(oid = 2))]
struct Type9;

#[derive(SqlType)]
#[diesel(postgres_type(oid = 1, array_oid = "1"))]
struct Type10;

#[derive(SqlType)]
#[diesel(postgres_type(oid = "1", array_oid = 1))]
struct Type11;

#[derive(SqlType)]
#[diesel(postgres_type(schema = "foo"))]
struct Type12;

#[derive(SqlType)]
#[diesel(postgres_type(what))]
struct Type13;

#[derive(SqlType)]
#[diesel(postgres_type(schema))]
struct Type14;

#[derive(SqlType)]
#[diesel(postgres_type(oid))]
struct Type15;

#[derive(SqlType)]
#[diesel(postgres_type(array_oid))]
struct Type16;

fn main() {}
