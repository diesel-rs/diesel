CREATE SCHEMA other;

SET SCHEMA 'other';
CREATE TABLE users(id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, hair_color TEXT);

CREATE VIEW test2 AS SELECT * FROM users;

SET SCHEMA 'public';

CREATE VIEW public.test AS SELECT * FROM other.test2;
