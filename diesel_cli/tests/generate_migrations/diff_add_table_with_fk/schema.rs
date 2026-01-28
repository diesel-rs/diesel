diesel::table! {
    users(user_id) {
        user_id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    posts(post_id) {
        post_id -> Integer,
        title -> Text,
        body -> Nullable<Text>,
        foreign_key_user_id -> Integer,
    }
}

diesel::joinable!(posts -> users (foreign_key_user_id));
