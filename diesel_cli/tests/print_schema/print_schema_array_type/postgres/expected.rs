// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int4,
        tags -> Array<Nullable<Text>>,
    }
}
