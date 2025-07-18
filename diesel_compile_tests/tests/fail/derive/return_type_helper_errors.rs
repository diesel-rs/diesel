extern crate diesel;

use diesel::{declare_sql_function, sql_types::SingleValue};

trait TypeWrapper {
    type Type: SingleValue;
}

impl<T: SingleValue> TypeWrapper for T {
    type Type = T;
}

mod with_return_type_helpers {
    use super::*;

    #[declare_sql_function(generate_return_type_helpers)]
    //~^ ERROR: expected `=`, the correct format is `generate_return_type_helpers = true/false`
    extern "SQL" {
        fn f<A: SingleValue>(a: <A as TypeWrapper>::Type);
        //~^ ERROR: cannot find argument corresponding to the generic

        #[variadic(1)]
        fn g<A: SingleValue>(a: <A as TypeWrapper>::Type);
        //~^ ERROR: cannot find argument corresponding to the generic

        #[skip_return_type_helper]
        fn h<A: SingleValue>(a: <A as TypeWrapper>::Type);

        #[skip_return_type_helper]
        #[variadic(1)]
        fn i<A: SingleValue>(a: <A as TypeWrapper>::Type);
    }
}

mod without_return_type_helpers {
    use super::*;

    #[declare_sql_function]
    extern "SQL" {
        fn f<A: SingleValue>(a: <A as TypeWrapper>::Type);

        #[variadic(1)]
        fn g<A: SingleValue>(a: <A as TypeWrapper>::Type);
    }
}

fn main() {}
