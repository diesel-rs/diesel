-- Three related tables.
CREATE TABLE users (id INT PRIMARY KEY);
CREATE TABLE posts (id INT PRIMARY KEY, user_id INT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id));
CREATE TABLE comments (id INT PRIMARY KEY, post_id INT NOT NULL, FOREIGN KEY (post_id) REFERENCES posts(id));

-- Two related tables.
CREATE TABLE sessions (id INT PRIMARY KEY);
CREATE TABLE transactions (id INT PRIMARY KEY, session_id INT NOT NULL, FOREIGN KEY (session_id) REFERENCES sessions(id));

-- Unrelated tables.
CREATE TABLE cars (id INT PRIMARY KEY);
CREATE TABLE bikes (id INT PRIMARY KEY);
