// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "language"))]
    pub struct Language;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Language;

    translations (word_id, translation_id) {
        word_id -> Int4,
        translation_id -> Int4,
        language -> Language,
    }
}
