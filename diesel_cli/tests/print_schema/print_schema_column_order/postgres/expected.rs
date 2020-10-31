// @generated automatically by Diesel CLI.

diesel::table! {
    abc (a) {
        a -> Int4,
        b -> Varchar,
        c -> Nullable<Bool>,
    }
}
