extern crate diesel;

use diesel::declare_sql_function;

mod named_parameters_block_error {
    use super::*;

    #[declare_sql_function(named_parameters)]
    //~^ ERROR: expected `=`, the correct format is `generate_return_type_helpers = true/false, named_parameters = true/false`
    extern "SQL" {
        fn a();
    }
}

mod named_parameters_fn_error {
    use super::*;

    #[declare_sql_function]
    extern "SQL" {
        #[named_parameters]
        //~^ ERROR: cannot find attribute `named_parameters` in this scope
        fn a();
    }
}

fn main() {}
