// @generated automatically by Diesel CLI.

diesel::table! {
    tab_key1 (id) {
        id -> Integer,
    }
}

diesel::table! {
    tab_problem (id) {
        id -> Integer,
        key1 -> BigInt,
    }
}

diesel::joinable!(tab_problem -> tab_key1 (key1));

diesel::allow_tables_to_appear_in_same_query!(
    tab_key1,
    tab_problem,
);
