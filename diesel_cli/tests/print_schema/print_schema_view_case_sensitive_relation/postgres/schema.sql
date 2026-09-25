CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE nullable_names (
    name TEXT
);
INSERT INTO nullable_names VALUES (NULL);

CREATE MATERIALIZED VIEW "Users" AS
SELECT nullable_names.name
FROM nullable_names;

CREATE VIEW v AS
SELECT "Users".name
FROM "Users";
