#[macro_use]
extern crate diesel;
use diesel::expression::AsExpression;

#[derive(Debug, AsExpression)]
#[sql_type = "foo"]
struct Lol1;

#[derive(AsExpression)]
#[sql_type]
struct Lol2;

#[derive(AsExpression)]
#[sql_type()]
struct Lol3;

#[derive(AsExpression)]
#[sql_type = 1]
struct Lol4;

#[derive(AsExpression)]
#[sql_type = "1"]
struct Lol5;

#[derive(QueryableByName)]
struct Lul1 {
    #[sql_type = "foo"]
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul2 {
    #[sql_type]
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul3 {
    #[sql_type()]
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul4 {
    #[sql_type = 1]
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul5 {
    #[sql_type = "1"]
    foo: i32,
}

fn main() {}
