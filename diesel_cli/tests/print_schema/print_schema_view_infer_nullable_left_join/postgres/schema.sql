CREATE TABLE users(id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, hair_color TEXT);
CREATE TABLE posts(id INTEGER PRIMARY KEY NOT NULL, user_id INTEGER NOT NULL, title TEXT NOT NULL, body TEXT);

CREATE VIEW test AS
SELECT
    users.id AS user_id,
    users.name AS user_name,
    users.hair_color AS user_hair_color,
    posts.id AS post_id,
    posts.title AS post_title,
    posts.body AS post_body
FROM users LEFT JOIN posts ON posts.user_id = users.id;
