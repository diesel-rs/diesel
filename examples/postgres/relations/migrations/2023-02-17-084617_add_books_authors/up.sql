-- Your SQL goes here
CREATE TABLE books_authors (
  book_id INTEGER REFERENCES books(id),
  author_id INTEGER REFERENCES authors(id),
  PRIMARY KEY(book_id, author_id)
);
