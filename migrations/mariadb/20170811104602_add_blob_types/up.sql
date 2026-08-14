CREATE TABLE all_the_blobs (
    id INTEGER PRIMARY KEY, -- Can't use a blob as a pk
    tiny TINYBLOB NOT NULL,
    normal BLOB NOT NULL,
    medium MEDIUMBLOB NOT NULL,
    big LONGBLOB NOT NULL
)
