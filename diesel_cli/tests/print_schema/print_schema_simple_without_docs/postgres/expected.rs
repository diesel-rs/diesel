table! {
    users1 (id) {
        id -> Int4,
    }
}

table! {
    users2 (id) {
        id -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(
    users1,
    users2,
);
