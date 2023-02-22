-- Your SQL goes here
CREATE TABLE pages (
  id SERIAL PRIMARY KEY,
  page_number INT NOT NULL,
  content TEXT NOT NULL,
  book_id SERIAL REFERENCES books(id)
);
