CREATE TABLE users (id INTEGER PRIMARY KEY, name VARCHAR(50) NOT NULL, tags TEXT[]);

CREATE MATERIALIZED VIEW user_names AS SELECT id, name, tags FROM users;
COMMENT ON MATERIALIZED VIEW user_names IS 'Cached user names';
COMMENT ON COLUMN user_names.name IS 'The name of the user';
