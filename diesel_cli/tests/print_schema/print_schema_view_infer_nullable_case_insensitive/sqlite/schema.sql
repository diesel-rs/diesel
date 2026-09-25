CREATE TABLE users(id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, hair_color TEXT);

-- SQLite matches names case-insensitively, so all of these refer to `users` and its columns
CREATE VIEW test AS
SELECT USERS.*, Users.NAME AS upper_name, hair_COLOR AS mixed_hair_color
FROM Users;
