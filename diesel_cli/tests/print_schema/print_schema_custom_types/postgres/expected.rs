// @generated automatically by Diesel CLI.

diesel::table! {
    use foo::*;
    use bar::*;

    users1 (id) {
        id -> Int4,
    }
}

diesel::table! {
    use foo::*;
    use bar::*;

    users2 (id) {
        id -> Int4,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    users1,
    users2,
);
