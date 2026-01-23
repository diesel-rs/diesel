-- Three related tables.
CREATE TABLE users (id INT PRIMARY KEY);
CREATE TABLE posts (id INT PRIMARY KEY, user_id INT NOT NULL REFERENCES users(id));
CREATE TABLE comments (id INT PRIMARY KEY, post_id INT NOT NULL REFERENCES posts(id));

-- Two related tables.
CREATE TABLE sessions (id INT PRIMARY KEY);
CREATE TABLE transactions (id INT PRIMARY KEY, session_id INT NOT NULL REFERENCES sessions(id));

-- Unrelated tables.
CREATE TABLE cars (id INT PRIMARY KEY);
CREATE TABLE bikes (id INT PRIMARY KEY);
