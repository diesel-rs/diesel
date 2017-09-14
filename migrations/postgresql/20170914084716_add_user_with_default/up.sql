CREATE TABLE users_with_default (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  hair_color VARCHAR DEFAULT 'Green'
);
