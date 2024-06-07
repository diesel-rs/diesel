-- Your SQL goes here
CREATE TABLE pages (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  page_number INT NOT NULL,
  content TEXT NOT NULL,
  book_id INTEGER NOT NULL REFERENCES books(id)
);
