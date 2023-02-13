diesel::table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    posts {
        id -> Integer,
        title -> Text,
        body -> Nullable<Text>,
        user_id -> Integer,
    }
}

diesel::joinable!(posts -> users (user_id));
