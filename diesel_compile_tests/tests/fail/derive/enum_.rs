//@no-rustfix
use diesel::prelude::*;
use diesel::types::Enum;

#[derive(Debug, Enum)]
enum Test {
    //~^ ERROR: no `#[diesel(sql_type = ...)]` attribute provided
    A,
}

#[derive(Debug, Enum)]
#[diesel(sql_type = diesel::sql_type::Integer)]
#[diesel(rename_all = "Foo")]
//~^ ERROR: got invalid case identifier: `Foo`
enum Test1 {
    A,
}

fn main() {}
