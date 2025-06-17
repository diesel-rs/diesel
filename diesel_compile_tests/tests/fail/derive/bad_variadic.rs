extern crate diesel;

use diesel::*;

#[declare_sql_function]
extern "SQL" {
    #[variadic(not_a_literal_number)]
    //~^ ERROR: expected integer literal
    fn f();

    #[variadic(3)]
    //~^ ERROR: invalid variadic argument count: not enough function arguments
    fn g<A: SqlType>(a: A);
}

fn main() {}
