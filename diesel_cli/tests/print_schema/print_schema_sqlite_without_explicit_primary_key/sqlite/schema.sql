CREATE TABLE no_explicit (
    name TEXT
);

CREATE TABLE with_explicit_rowid (
    name TEXT,
    rowid TEXT
);

CREATE TABLE with_explicit_rowid_oid (
    name TEXT,
    rowid TEXT,
    oid TEXT
);

CREATE TABLE with_explicit_pk_rowid (
    rowid INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE with_explicit_pk_rowid_autoincrement (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT
);

CREATE TABLE with_explicit_pk_rowid_not_null (
    rowid INTEGER PRIMARY KEY NOT NULL,
    name TEXT
);

CREATE TABLE with_explicit_pk_rowid_autoincrement_not_null (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT
);

CREATE TABLE with_explicit_aliased_rowid (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE with_explicit_aliased_rowid_not_null (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT
);

CREATE TABLE without_rowid (
    word TEXT PRIMARY KEY,
    cnt INTEGER
) WITHOUT ROWID;
