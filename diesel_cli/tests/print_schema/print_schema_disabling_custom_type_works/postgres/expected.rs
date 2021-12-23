// @generated automatically by Diesel CLI.

diesel::table! {
    custom_types (id) {
        id -> Int4,
        custom_enum -> MyType,
    }
}
