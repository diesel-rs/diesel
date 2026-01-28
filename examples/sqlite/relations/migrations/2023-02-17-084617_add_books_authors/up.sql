-- Your SQL goes here
CREATE TABLE books_authors (
  book_id INTEGER NOT NULL REFERENCES books(id),
  author_id INTEGER NOT NULL REFERENCES authors(id),
  PRIMARY KEY(book_id, author_id)
);
