table! {
    all_the_ranges (int4) {
        int4 -> Int4range,
        int8 -> Int8range,
        num -> Numrange,
        ts -> Tsrange,
        tstz -> Tstzrange,
        date -> Daterange,
    }
}

table! {
    comments (id) {
        id -> Int4,
        post_id -> Int4,
        text -> Text,
    }
}

table! {
    composite_fk (id) {
        id -> Int4,
        post_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    cyclic_fk_1 (id) {
        id -> Int4,
        cyclic_fk_2_id -> Nullable<Int4>,
    }
}

table! {
    cyclic_fk_2 (id) {
        id -> Int4,
        cyclic_fk_1_id -> Nullable<Int4>,
    }
}

table! {
    fk_doesnt_reference_pk (id) {
        id -> Int4,
        random -> Nullable<Text>,
    }
}

table! {
    fk_inits (id) {
        id -> Int4,
    }
}

table! {
    fk_tests (id) {
        id -> Int4,
        fk_id -> Int4,
    }
}

table! {
    followings (user_id, post_id) {
        user_id -> Int4,
        post_id -> Int4,
        email_notifications -> Bool,
    }
}

table! {
    likes (comment_id, user_id) {
        comment_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    multiple_fks_to_same_table (id) {
        id -> Int4,
        post_id_1 -> Nullable<Int4>,
        post_id_2 -> Nullable<Int4>,
    }
}

table! {
    nullable_doubles (id) {
        id -> Int4,
        n -> Nullable<Float8>,
    }
}

table! {
    nullable_table (id) {
        id -> Int4,
        value -> Nullable<Int4>,
    }
}

table! {
    numbers (n) {
        n -> Int4,
    }
}

table! {
    points (x, y) {
        x -> Int4,
        y -> Int4,
    }
}

table! {
    pokes (user_id) {
        user_id -> Int4,
        poke_count -> Int4,
    }
}

table! {
    posts (id) {
        id -> Int4,
        user_id -> Int4,
        title -> Varchar,
        body -> Nullable<Text>,
        tags -> Array<Text>,
    }
}

table! {
    precision_numbers (n) {
        n -> Float8,
    }
}

table! {
    self_referential_fk (id) {
        id -> Int4,
        parent_id -> Int4,
    }
}

table! {
    special_comments (id) {
        id -> Int4,
        special_post_id -> Int4,
    }
}

table! {
    special_posts (id) {
        id -> Int4,
        user_id -> Int4,
        title -> Varchar,
    }
}

table! {
    trees (id) {
        id -> Int4,
        parent_id -> Nullable<Int4>,
    }
}

table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        hair_color -> Nullable<Varchar>,
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
        fn_ -> Int4,
        #[sql_name = "let"]
        let_ -> Int4,
        #[sql_name = "extern"]
        extern_ -> Int4,
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
    all_the_ranges,
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
    users,
    users_with_name_pk,
    with_keywords,
);
