extern crate diesel;

use diesel::*;

#[declare_sql_function]
extern "SQL" {
    //~^ ERROR: invalid ABI: found `SQL`

    #[variadic(not_a_literal_number)]
    //~^ ERROR: expect `last_arguments`, the correct format is `#[variadic(last_arguments = 3)]` or `#[variadic(last_arguments = 3, skip_zero_argument_variant = true)]`
    //~| ERROR: cannot find attribute `variadic` in this scope
    fn f();
}

#[declare_sql_function]
extern "SQL" {
    #[variadic(last_arguments = 3)]
    //~^ ERROR: invalid variadic argument count: not enough function arguments
    fn g<A: SqlType>(a: A);
}

fn main() {}
