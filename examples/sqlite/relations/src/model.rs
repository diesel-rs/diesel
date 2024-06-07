use diesel::prelude::*;

use crate::schema::{authors, books, books_authors, pages};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug, Clone)]
#[diesel(table_name = authors)]
pub struct Author {
    pub id: i32,
    pub name: String,
}

#[derive(Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(Book))]
#[diesel(belongs_to(Author))]
#[diesel(table_name = books_authors)]
#[diesel(primary_key(book_id, author_id))]
pub struct BookAuthor {
    pub book_id: i32,
    pub author_id: i32,
}

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = books)]
pub struct Book {
    pub id: i32,
    pub title: String,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Book))]
#[diesel(table_name = pages)]
pub struct Page {
    pub id: i32,
    pub page_number: i32,
    pub content: String,
    pub book_id: i32,
}
