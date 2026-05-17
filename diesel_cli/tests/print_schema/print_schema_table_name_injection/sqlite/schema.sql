CREATE TABLE parent_table (id INTEGER PRIMARY KEY);

CREATE TABLE "quote'table" (
    id INTEGER PRIMARY KEY,
    name TEXT,
    parent_id INTEGER NOT NULL REFERENCES parent_table(id)
);
