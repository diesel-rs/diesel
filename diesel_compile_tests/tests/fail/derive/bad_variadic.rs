extern crate diesel;

use diesel::*;

#[declare_sql_function]
extern "SQL" {
    #[variadic(not_a_literal_number)]
    fn f();

    #[variadic(3)]
    fn g<A: SqlType>(a: A);
}

fn main() {}
