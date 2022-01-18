CREATE TABLE generated (
    id integer primary key,
    generated integer generated always as (id * 3)
);
