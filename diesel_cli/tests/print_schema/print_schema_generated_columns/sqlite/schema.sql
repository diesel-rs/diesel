CREATE TABLE generated (
    id integer primary key,
    generated integer as (id * 3)
);
