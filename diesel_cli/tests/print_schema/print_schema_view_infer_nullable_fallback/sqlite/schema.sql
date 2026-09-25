CREATE TABLE users(id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL);

-- SQLite accepts the schema name, but print-schema cannot resolve tables through it
CREATE VIEW names AS SELECT name FROM main.users;

-- nor the wildcards of such a view
CREATE VIEW all_users AS SELECT * FROM main.users;

-- SQLite's rowid is no column of the table
CREATE VIEW row_ids AS SELECT rowid AS r, name FROM users;

-- inference cannot tell whether SQLite reads `u` as `(id IS NOT DISTINCT FROM name) = name`,
-- so `u` keeps the nullability the database reports, while `id` gets the inferred one
CREATE VIEW unknown AS SELECT id IS NOT DISTINCT FROM name = name AS u, id FROM users;
