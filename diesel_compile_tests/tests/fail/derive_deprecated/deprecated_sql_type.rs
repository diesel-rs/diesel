#[macro_use]
extern crate diesel;
use diesel::expression::AsExpression;

#[derive(Debug, AsExpression)]
#[sql_type = "foo"]
//~^ ERROR: cannot find type `foo` in this scope
struct Lol1;

#[derive(AsExpression)]
#[sql_type]
//~^ ERROR: unexpected end of input, expected `=`
struct Lol2;

#[derive(AsExpression)]
#[sql_type()]
//~^ ERROR: expected `=`
struct Lol3;

#[derive(AsExpression)]
#[sql_type = 1]
//~^ ERROR: expected string literal
struct Lol4;

#[derive(AsExpression)]
#[sql_type = "1"]
//~^ ERROR: expected identifier
struct Lol5;

#[derive(QueryableByName)]
struct Lul1 {
    #[sql_type = "foo"]
    //~^ ERROR: cannot find type `foo` in this scope
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul2 {
    #[sql_type]
    //~^ ERROR: unexpected end of input, expected `=`
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul3 {
    #[sql_type()]
    //~^ ERROR: expected `=
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul4 {
    #[sql_type = 1]
    //~^ ERROR: expected string literal
    foo: i32,
}

#[derive(QueryableByName)]
struct Lul5 {
    #[sql_type = "1"]
    //~^ ERROR: expected identifier
    foo: i32,
}

fn main() {}
