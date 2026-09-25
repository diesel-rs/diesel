CREATE SCHEMA other;

CREATE TABLE public.users (
    id INTEGER PRIMARY KEY,
    name TEXT
);
INSERT INTO public.users VALUES (1, NULL);

CREATE TABLE other.users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE VIEW other.good AS
SELECT users.name
FROM users;

-- the search path finds `pg_description` in `pg_catalog`, not in the default schema
CREATE VIEW other.descriptions AS
SELECT description
FROM pg_description;
