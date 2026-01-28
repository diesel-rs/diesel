-- Three related tables.
CREATE TABLE users (id INT PRIMARY KEY AUTO_INCREMENT);
CREATE TABLE posts (id INT PRIMARY KEY AUTO_INCREMENT, user_id INT NOT NULL, FOREIGN KEY (user_id) REFERENCES users(id));
CREATE TABLE comments (
    id INT PRIMARY KEY AUTO_INCREMENT,
    post_id INT NOT NULL,
    post_id_2 INT NOT NULL,
    CONSTRAINT fk_comments_post_id FOREIGN KEY (post_id) REFERENCES posts(id),
    CONSTRAINT fk_comments_post_id_2 FOREIGN KEY (post_id_2) REFERENCES posts(id)
);

-- Two related tables.
CREATE TABLE sessions (id INT PRIMARY KEY AUTO_INCREMENT);
CREATE TABLE transactions (id INT PRIMARY KEY AUTO_INCREMENT, session_id INT NOT NULL, FOREIGN KEY (session_id) REFERENCES sessions(id));

-- Unrelated tables.
CREATE TABLE cars (id INT PRIMARY KEY AUTO_INCREMENT);
CREATE TABLE bikes (id INT PRIMARY KEY AUTO_INCREMENT);
