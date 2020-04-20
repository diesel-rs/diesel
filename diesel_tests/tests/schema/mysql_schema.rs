table! {
    all_the_blobs (id) {
        id -> Integer,
        tiny -> Tinyblob,
        normal -> Blob,
        medium -> Mediumblob,
        big -> Longblob,
    }
}

table! {
    comments (id) {
        id -> Integer,
        post_id -> Integer,
        text -> Text,
    }
}

table! {
    composite_fk (id) {
        id -> Integer,
        post_id -> Integer,
        user_id -> Integer,
    }
}

table! {
    cyclic_fk_1 (id) {
        id -> Integer,
        cyclic_fk_2_id -> Nullable<Integer>,
    }
}

table! {
    cyclic_fk_2 (id) {
        id -> Integer,
        cyclic_fk_1_id -> Nullable<Integer>,
    }
}

table! {
    fk_doesnt_reference_pk (id) {
        id -> Integer,
        random -> Nullable<Text>,
    }
}

table! {
    fk_inits (id) {
        id -> Integer,
    }
}

table! {
    fk_tests (id) {
        id -> Integer,
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
    likes (comment_id, user_id) {
        comment_id -> Integer,
        user_id -> Integer,
    }
}

table! {
    multiple_fks_to_same_table (id) {
        id -> Integer,
        post_id_1 -> Nullable<Integer>,
        post_id_2 -> Nullable<Integer>,
    }
}

table! {
    nullable_doubles (id) {
        id -> Integer,
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
        n -> Integer,
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
        id -> Integer,
        parent_id -> Integer,
    }
}

table! {
    special_comments (id) {
        id -> Integer,
        special_post_id -> Integer,
    }
}

table! {
    special_posts (id) {
        id -> Integer,
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
    unsigned_table (id) {
        id -> Unsigned<Integer>,
        value -> Unsigned<Integer>,
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
        name -> Varchar,
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
joinable!(cyclic_fk_1 -> cyclic_fk_2 (cyclic_fk_2_id));
joinable!(fk_tests -> fk_inits (fk_id));
joinable!(followings -> posts (post_id));
joinable!(followings -> users (user_id));
joinable!(likes -> comments (comment_id));
joinable!(likes -> users (user_id));
joinable!(posts -> users (user_id));

allow_tables_to_appear_in_same_query!(
    all_the_blobs,
    comments,
    composite_fk,
    cyclic_fk_1,
    cyclic_fk_2,
    fk_doesnt_reference_pk,
    fk_inits,
    fk_tests,
    followings,
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
    unsigned_table,
    users,
    users_with_name_pk,
    with_keywords,
);
