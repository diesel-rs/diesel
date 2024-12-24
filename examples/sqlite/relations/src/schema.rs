// @generated automatically by Diesel CLI.

diesel::table! {
    authors (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    books (id) {
        id -> Integer,
        title -> Text,
    }
}

diesel::table! {
    books_authors (book_id, author_id) {
        book_id -> Integer,
        author_id -> Integer,
    }
}

diesel::table! {
    pages (id) {
        id -> Integer,
        page_number -> Integer,
        content -> Text,
        book_id -> Integer,
    }
}

diesel::joinable!(books_authors -> authors (author_id));
diesel::joinable!(books_authors -> books (book_id));
diesel::joinable!(pages -> books (book_id));

diesel::allow_tables_to_appear_in_same_query!(authors, books, books_authors, pages,);
