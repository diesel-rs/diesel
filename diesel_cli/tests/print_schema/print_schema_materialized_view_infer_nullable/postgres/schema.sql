CREATE TABLE users (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, hair_color TEXT);

CREATE MATERIALIZED VIEW user_hair_colors AS SELECT id, hair_color FROM users;

CREATE VIEW user_ids AS SELECT id FROM user_hair_colors;
