//! Test submodule to verify that the `allow_tables_to_appear_in_same_query!` macro
//! can accept as input tables defined in the root module as well as in submodules.
use diesel::allow_tables_to_appear_in_same_query;
use diesel::table;

table! {
    table_a (id) {
        id -> Integer,
    }
}

mod sub_table {
    use diesel::table;
    table! {
        table_b (id) {
            id -> Integer,
        }
    }
}

table! {
    table_c (id) {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(table_a, table_c);
allow_tables_to_appear_in_same_query!(table_a, sub_table::table_b);

