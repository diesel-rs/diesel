// @generated automatically by Diesel CLI.

diesel::table! {
    /// Representation of the `posts` table.
    ///
    /// (Automatically generated by Diesel.)
    posts (id) {
        /// The `id` column of the `posts` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int4,
        /// The `type` column of the `posts` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        #[sql_name = "type"]
        type_ -> Int4,
    }
}

diesel::table! {
    /// Representation of the `users` table.
    ///
    /// (Automatically generated by Diesel.)
    users (id) {
        /// The `id` column of the `users` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int4,
    }
}

diesel::joinable!(posts -> users (type_));

diesel::allow_tables_to_appear_in_same_query!(
    posts,
    users,
);
