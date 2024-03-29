use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use std::error::Error;

pub mod model;
pub mod schema;

use crate::model::*;
use crate::schema::*;

fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {database_url}"))
}

fn new_author(conn: &mut PgConnection, name: &str) -> Result<Author, Box<dyn Error + Send + Sync>> {
    let author = diesel::insert_into(authors::table)
        .values(authors::name.eq(name))
        .returning(Author::as_returning())
        .get_result(conn)?;
    Ok(author)
}

fn new_book(conn: &mut PgConnection, title: &str) -> Result<Book, Box<dyn Error + Send + Sync>> {
    let book = diesel::insert_into(books::table)
        .values(books::title.eq(title))
        .returning(Book::as_returning())
        .get_result(conn)?;
    Ok(book)
}

fn new_books_author(
    conn: &mut PgConnection,
    book_id: i32,
    author_id: i32,
) -> Result<BookAuthor, Box<dyn Error + Send + Sync>> {
    let book_author = diesel::insert_into(books_authors::table)
        .values((
            books_authors::book_id.eq(book_id),
            books_authors::author_id.eq(author_id),
        ))
        .returning(BookAuthor::as_returning())
        .get_result(conn)?;
    Ok(book_author)
}

fn new_page(
    conn: &mut PgConnection,
    page_number: i32,
    content: &str,
    book_id: i32,
) -> Result<Page, Box<dyn Error + Send + Sync>> {
    let page = diesel::insert_into(pages::table)
        .values((
            pages::page_number.eq(page_number),
            pages::content.eq(content),
            pages::book_id.eq(book_id),
        ))
        .returning(Page::as_returning())
        .get_result(conn)?;
    Ok(page)
}

fn joins(conn: &mut PgConnection) -> Result<(), Box<dyn Error + Send + Sync>> {
    let page_with_book = pages::table
        .inner_join(books::table)
        .filter(books::title.eq("Momo"))
        .select((Page::as_select(), Book::as_select()))
        .load::<(Page, Book)>(conn)?;

    println!("Page-Book pairs: {page_with_book:?}");

    let book_without_pages = books::table
        .left_join(pages::table)
        .select((Book::as_select(), Option::<Page>::as_select()))
        .load::<(Book, Option<Page>)>(conn)?;

    println!("Book-Page pairs (including empty books): {book_without_pages:?}");
    Ok(())
}

fn one_to_n_relations(conn: &mut PgConnection) -> Result<(), Box<dyn Error + Send + Sync>> {
    let momo = books::table
        .filter(books::title.eq("Momo"))
        .select(Book::as_select())
        .get_result(conn)?;

    // get pages for the book "Momo"
    let pages = Page::belonging_to(&momo)
        .select(Page::as_select())
        .load(conn)?;

    println!("Pages for \"Momo\": \n {pages:?}\n");

    let all_books = books::table.select(Book::as_select()).load(conn)?;

    // get all pages for all books
    let pages = Page::belonging_to(&all_books)
        .select(Page::as_select())
        .load(conn)?;

    // group the pages per book
    let pages_per_book = pages
        .grouped_by(&all_books)
        .into_iter()
        .zip(all_books)
        .map(|(pages, book)| (book, pages))
        .collect::<Vec<(Book, Vec<Page>)>>();

    println!("Pages per book: \n {pages_per_book:?}\n");

    Ok(())
}

fn m_to_n_relations(conn: &mut PgConnection) -> Result<(), Box<dyn Error + Send + Sync>> {
    let astrid_lindgren = authors::table
        .filter(authors::name.eq("Astrid Lindgren"))
        .select(Author::as_select())
        .get_result(conn)?;

    // get all of Astrid Lindgren's books
    let books = BookAuthor::belonging_to(&astrid_lindgren)
        .inner_join(books::table)
        .select(Book::as_select())
        .load(conn)?;
    println!("Asgrid Lindgren books: {books:?}");

    let collaboration = books::table
        .filter(books::title.eq("Pippi and Momo"))
        .select(Book::as_select())
        .get_result(conn)?;

    // get authors for the collaboration
    let authors = BookAuthor::belonging_to(&collaboration)
        .inner_join(authors::table)
        .select(Author::as_select())
        .load(conn)?;
    println!("Authors for \"Pipi and Momo\": {authors:?}");

    // get a list of authors with all their books
    let all_authors = authors::table.select(Author::as_select()).load(conn)?;

    let books = BookAuthor::belonging_to(&authors)
        .inner_join(books::table)
        .select((BookAuthor::as_select(), Book::as_select()))
        .load(conn)?;

    let books_per_author: Vec<(Author, Vec<Book>)> = books
        .grouped_by(&all_authors)
        .into_iter()
        .zip(authors)
        .map(|(b, author)| (author, b.into_iter().map(|(_, book)| book).collect()))
        .collect();

    println!("All authors including their books: {books_per_author:?}");

    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let conn = &mut establish_connection();
    setup_data(conn)?;

    one_to_n_relations(conn)?;
    joins(conn)?;
    m_to_n_relations(conn)?;
    Ok(())
}

fn setup_data(conn: &mut PgConnection) -> Result<(), Box<dyn Error + Send + Sync>> {
    // create a book
    let momo = new_book(conn, "Momo")?;

    // a page in that book
    new_page(conn, 1, "In alten, alten Zeiten ...", momo.id)?;
    // a second page
    new_page(conn, 2, "den prachtvollen Theatern...", momo.id)?;

    // create an author
    let michael_ende = new_author(conn, "Michael Ende")?;

    // let's add the author to the already created book
    new_books_author(conn, momo.id, michael_ende.id)?;

    // create a second author
    let astrid_lindgren = new_author(conn, "Astrid Lindgren")?;
    let pippi = new_book(conn, "Pippi Långstrump")?;
    new_books_author(conn, pippi.id, astrid_lindgren.id)?;

    // now that both have a single book, let's add a third book, an imaginary collaboration
    let collaboration = new_book(conn, "Pippi and Momo")?;
    new_books_author(conn, collaboration.id, astrid_lindgren.id)?;
    new_books_author(conn, collaboration.id, michael_ende.id)?;

    Ok(())
}
