table! {
    use foo::*;
    use bar::*;

    users1 (id) {
        id -> Integer,
    }
}

table! {
    use foo::*;
    use bar::*;

    users2 (id) {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(
    users1,
    users2,
);
