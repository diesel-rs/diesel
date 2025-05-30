extern crate diesel;

use diesel::{declare_sql_function, sql_types::SingleValue};

trait TypeWrapper {
    type Type: SingleValue;
}

impl<T: SingleValue> TypeWrapper for T {
    type Type = T;
}

#[declare_sql_function]
extern "SQL" {
    fn f<A: SingleValue>(a: <A as TypeWrapper>::Type);

    #[skip_return_type_helper]
    fn g<A: SingleValue>(a: <A as TypeWrapper>::Type);
}

fn main() {}
