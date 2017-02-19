CREATE TABLE fk_inits (
    id INTEGER PRIMARY KEY
);

CREATE TABLE fk_tests (
    id INTEGER PRIMARY KEY,
    fk_id INTEGER NOT NULL REFERENCES fk_inits(id)
);
