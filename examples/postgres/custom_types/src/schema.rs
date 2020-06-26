diesel::table! {
    use diesel::sql_types::*;
    use crate::model::exports::*;

    translations (word_id, translation_id) {
        word_id -> Int4,
        translation_id -> Int4,
        language -> Language,
    }
}
