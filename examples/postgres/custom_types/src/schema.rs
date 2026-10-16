// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "language"))]
    #[diesel(enum_type)]
    pub struct Language;
}

pub mod rust_types {
    #[derive(Debug, diesel::types::Enum)]
    #[diesel(sql_type = super::sql_types::Language)]
    pub enum Language {
        #[diesel(rename = "en")]
        En,
        #[diesel(rename = "ru")]
        Ru,
        #[diesel(rename = "de")]
        De,
    }
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
