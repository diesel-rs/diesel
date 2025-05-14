extern crate diesel;

use diesel::*;

#[declare_sql_function]
extern "SQL" {
    #[variadic(non_a_literal_number)]
    fn f();

    #[variadic(3)]
    fn g<A: SqlType>(a: A);

    #[variadic(1)]
    fn h(a: impl SqlType);
}

fn main() {}
