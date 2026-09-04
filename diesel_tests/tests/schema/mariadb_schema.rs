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
        #[auto_increment]
        id -> Integer,
        post_id -> Integer,
        text -> Text,
    }
}

table! {
    composite_fk (id) {
        #[auto_increment]
        id -> Integer,
        post_id -> Integer,
        user_id -> Integer,
    }
}

table! {
    cyclic_fk_1 (id) {
        #[auto_increment]
        id -> Integer,
        cyclic_fk_2_id -> Nullable<Integer>,
    }
}

table! {
    cyclic_fk_2 (id) {
        #[auto_increment]
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
        #[auto_increment]
        id -> Integer,
        post_id_1 -> Nullable<Integer>,
        post_id_2 -> Nullable<Integer>,
    }
}

table! {
    nullable_doubles (id) {
        #[auto_increment]
        id -> Integer,
        n -> Nullable<Double>,
    }
}

table! {
    nullable_table (id) {
        #[auto_increment]
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
        #[auto_increment]
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
        #[auto_increment]
        id -> Integer,
        parent_id -> Integer,
    }
}

table! {
    special_comments (id) {
        #[auto_increment]
        id -> Integer,
        special_post_id -> Integer,
    }
}

table! {
    special_posts (id) {
        #[auto_increment]
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
        #[auto_increment]
        id -> Unsigned<Integer>,
        value -> Unsigned<Integer>,
    }
}

table! {
    unsigned_widths (id) {
        #[auto_increment]
        id -> Unsigned<Integer>,
        tiny_value -> Unsigned<TinyInt>,
        small_value -> Unsigned<SmallInt>,
        int_value -> Unsigned<Integer>,
        big_value -> Unsigned<BigInt>,
    }
}

table! {
    users (id) {
        #[auto_increment]
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
    unsigned_widths,
    users,
    users_with_name_pk,
    with_keywords,
);
