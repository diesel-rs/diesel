table! {
    users1 (id) {
        id -> Integer,
    }
}

table! {
    users2 (myid) {
        #[sql_name = "id"]
        myid -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(
    users1,
    users2,
);
