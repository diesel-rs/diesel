-- Three related tables.
CREATE TABLE users (id INTEGER PRIMARY KEY);
CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id));
CREATE TABLE comments (
    id INTEGER PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES posts(id),
    post_id_2 INTEGER NOT NULL REFERENCES posts(id)
);

-- Two related tables.
CREATE TABLE sessions (id INTEGER PRIMARY KEY);
CREATE TABLE transactions (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id));

-- Unrelated tables.
CREATE TABLE cars (id INTEGER PRIMARY KEY);
CREATE TABLE bikes (id INTEGER PRIMARY KEY);
