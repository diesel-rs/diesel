use diesel::prelude::*;

table! {
    two_counters {
        #[auto_increment]
        id -> Integer,
        #[auto_increment]
        other -> Integer,
        //~^ ERROR: at most one column per table can be marked with `#[auto_increment]`
    }
}

diesel::view! {
    counted_view {
        #[auto_increment]
        id -> Integer,
        //~^ ERROR: `#[auto_increment]` is not supported in `view!` definitions
        name -> Text,
    }
}

fn main() {}
