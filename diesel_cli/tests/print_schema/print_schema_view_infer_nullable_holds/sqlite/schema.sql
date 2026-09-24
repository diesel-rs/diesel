CREATE TABLE users (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, bytes BLOB NOT NULL);
INSERT INTO users VALUES (1, 'a', x'FF'), (2, '', x'');
CREATE TABLE empty_table (id INTEGER NOT NULL);

-- SQLite turns any value into text
CREATE VIEW casts AS SELECT CAST(bytes AS TEXT) AS bytes_text, CAST(name AS INTEGER) AS name_number FROM users;
-- an application can replace the function SQLite evaluates LIKE with
CREATE VIEW patterns AS SELECT name LIKE 'a%' AS starts_with_a, name = 'a' AS is_a FROM users;
-- a division by zero is NULL
CREATE VIEW operators AS SELECT id / 0 AS ratio, id < 2 AS is_first FROM users;
-- an aggregate query without GROUP BY returns a row even for an empty table
CREATE VIEW aggregates AS SELECT id, count(*) AS n FROM empty_table;
