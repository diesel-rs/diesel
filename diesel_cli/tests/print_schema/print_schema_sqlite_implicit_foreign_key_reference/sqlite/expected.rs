// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Integer,
        account -> Text,
        data_center_id -> Integer,
        auth_key -> Binary,
    }
}

diesel::table! {
    data_centers (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::joinable!(accounts -> data_centers (data_center_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    data_centers,
);
