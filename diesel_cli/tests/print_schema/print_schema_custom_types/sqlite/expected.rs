table! {
    use foo::*;
    use bar::*;

    users1 (id) {
        id -> Nullable<Integer>,
    }
}

table! {
    use foo::*;
    use bar::*;

    users2 (id) {
        id -> Nullable<Integer>,
    }
}

allow_tables_to_appear_in_same_query!(
    users1,
    users2,
);
