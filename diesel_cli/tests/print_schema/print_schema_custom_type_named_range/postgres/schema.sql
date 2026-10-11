CREATE TYPE range AS ENUM ('ideal', 'rated');
CREATE TYPE unit_of_length AS ENUM ('km', 'mi');
CREATE TABLE settings (
    id SERIAL PRIMARY KEY,
    unit_of_length unit_of_length NOT NULL,
    preferred_range range NOT NULL,
    int_range int4range NOT NULL
);
