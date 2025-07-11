extern crate diesel;

use diesel::*;

#[declare_sql_function]
extern "SQL" {
    //~^ ERROR: invalid ABI: found `SQL`

    #[variadic(not_a_literal_number)]
    //~^ ERROR: expected integer literal, the correct format is `#[variadic(3)]`
    //~| ERROR: cannot find attribute `variadic` in this scope
    fn f();
}

#[declare_sql_function]
extern "SQL" {
    #[variadic(3)]
    //~^ ERROR: invalid variadic argument count: not enough function arguments
    fn g<A: SqlType>(a: A);
}

fn main() {}
