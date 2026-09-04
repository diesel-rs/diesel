// @generated automatically by Diesel CLI.

diesel::table! {
    posts (id) {
        #[auto_increment]
        id -> Integer,
        #[max_length = 255]
        title -> Varchar,
        body -> Text,
        published -> Bool,
    }
}
