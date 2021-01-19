// @generated automatically by Diesel CLI.

diesel::table! {
    tab_key1 (id) {
        id -> Bigint,
    }
}

diesel::table! {
    tab_problem (id) {
        id -> Bigint,
        key1 -> Bigint,
    }
}

diesel::joinable!(tab_problem -> tab_key1 (key1));

diesel::allow_tables_to_appear_in_same_query!(
    tab_key1,
    tab_problem,
);
