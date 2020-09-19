// @generated automatically by Diesel CLI.

diesel::table! {
    use foo::*;
    use bar::*;

    users1 (id) {
        id -> Integer,
    }
}

diesel::table! {
    use foo::*;
    use bar::*;

    users2 (id) {
        id -> Integer,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    users1,
    users2,
);
