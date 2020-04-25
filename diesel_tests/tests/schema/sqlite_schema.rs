table! {
    comments (id) {
        id -> Integer,
        post_id -> Integer,
        text -> Text,
    }
}

table! {
    composite_fk (id) {
        id -> Nullable<Integer>,
        post_id -> Integer,
        user_id -> Integer,
    }
}

table! {
    cyclic_fk_1 (id) {
        id -> Nullable<Integer>,
        cyclic_fk_2_id -> Nullable<Binary>,
    }
}

table! {
    cyclic_fk_2 (id) {
        id -> Nullable<Integer>,
        cyclic_fk_1_id -> Nullable<Binary>,
    }
}

table! {
    fk_doesnt_reference_pk (id) {
        id -> Nullable<Integer>,
        random -> Nullable<Text>,
    }
}

table! {
    fk_inits (id) {
        id -> Nullable<Integer>,
    }
}

table! {
    fk_tests (id) {
        id -> Nullable<Integer>,
        fk_id -> Integer,
    }
}

table! {
    followings (user_id, post_id) {
        user_id -> Integer,
        post_id -> Integer,
        email_notifications -> Bool,
    }
}

table! {
    infer_all_the_bools (col1) {
        col1 -> Bool,
        col2 -> Bool,
        col3 -> Bool,
        col4 -> Bool,
    }
}

table! {
    infer_all_the_datetime_types (dt) {
        dt -> Timestamp,
        date -> Date,
        time -> Time,
        timestamp -> Timestamp,
    }
}

table! {
    infer_all_the_floats (col1) {
        col1 -> Float,
        col2 -> Float,
        col3 -> Double,
        col4 -> Double,
        col5 -> Double,
        col6 -> Double,
    }
}

table! {
    infer_all_the_ints (col1) {
        col1 -> Integer,
        col2 -> Integer,
        col3 -> Integer,
        col4 -> Integer,
        col5 -> SmallInt,
        col6 -> SmallInt,
        col7 -> SmallInt,
        col8 -> BigInt,
        col9 -> BigInt,
        col10 -> BigInt,
        col11 -> SmallInt,
        col12 -> Integer,
        col13 -> BigInt,
    }
}

table! {
    infer_all_the_strings (col1) {
        col1 -> Text,
        col2 -> Text,
        col3 -> Text,
        col4 -> Text,
        col5 -> Text,
        col6 -> Text,
        col7 -> Text,
        col8 -> Text,
        col9 -> Binary,
        col10 -> Binary,
    }
}

table! {
    likes (comment_id, user_id) {
        comment_id -> Integer,
        user_id -> Integer,
    }
}

table! {
    multiple_fks_to_same_table (id) {
        id -> Nullable<Integer>,
        post_id_1 -> Nullable<Binary>,
        post_id_2 -> Nullable<Binary>,
    }
}

table! {
    nullable_doubles (id) {
        id -> Nullable<Integer>,
        n -> Nullable<Double>,
    }
}

table! {
    nullable_table (id) {
        id -> Integer,
        value -> Nullable<Integer>,
    }
}

table! {
    numbers (n) {
        n -> Nullable<Integer>,
    }
}

table! {
    points (x, y) {
        x -> Integer,
        y -> Integer,
    }
}

table! {
    pokes (user_id) {
        user_id -> Integer,
        poke_count -> Integer,
    }
}

table! {
    posts (id) {
        id -> Integer,
        user_id -> Integer,
        title -> Text,
        body -> Nullable<Text>,
    }
}

table! {
    precision_numbers (n) {
        n -> Double,
    }
}

table! {
    self_referential_fk (id) {
        id -> Nullable<Integer>,
        parent_id -> Integer,
    }
}

table! {
    special_comments (id) {
        id -> Nullable<Integer>,
        special_post_id -> Integer,
    }
}

table! {
    special_posts (id) {
        id -> Nullable<Integer>,
        user_id -> Integer,
        title -> Text,
    }
}

table! {
    trees (id) {
        id -> Integer,
        parent_id -> Nullable<Integer>,
    }
}

table! {
    users (id) {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    users_with_name_pk (name) {
        name -> Nullable<Text>,
    }
}

table! {
    with_keywords (fn_) {
        #[sql_name = "fn"]
        fn_ -> Integer,
        #[sql_name = "let"]
        let_ -> Integer,
        #[sql_name = "extern"]
        extern_ -> Integer,
    }
}

joinable!(comments -> posts (post_id));
joinable!(fk_tests -> fk_inits (fk_id));
joinable!(followings -> posts (post_id));
joinable!(followings -> users (user_id));
joinable!(likes -> comments (comment_id));
joinable!(likes -> users (user_id));
joinable!(pokes -> users (user_id));
joinable!(posts -> users (user_id));

allow_tables_to_appear_in_same_query!(
    comments,
    composite_fk,
    cyclic_fk_1,
    cyclic_fk_2,
    fk_doesnt_reference_pk,
    fk_inits,
    fk_tests,
    followings,
    infer_all_the_bools,
    infer_all_the_datetime_types,
    infer_all_the_floats,
    infer_all_the_ints,
    infer_all_the_strings,
    likes,
    multiple_fks_to_same_table,
    nullable_doubles,
    nullable_table,
    numbers,
    points,
    pokes,
    posts,
    precision_numbers,
    self_referential_fk,
    special_comments,
    special_posts,
    trees,
    users,
    users_with_name_pk,
    with_keywords,
);
