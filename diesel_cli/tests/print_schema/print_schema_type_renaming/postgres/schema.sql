CREATE TYPE user_job AS ENUM ('programmer', 'director', 'writer', 'mathematician');

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    job user_job NOT NULL
);

