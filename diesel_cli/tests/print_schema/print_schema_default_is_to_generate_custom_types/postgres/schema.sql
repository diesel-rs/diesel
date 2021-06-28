CREATE TYPE my_type AS ENUM ('foo', 'bar');
CREATE TYPE my_type2 AS ENUM ('foo', 'bar');
CREATE TABLE custom_types (
    id SERIAL PRIMARY KEY,
    custom_enum my_type NOT NULL,
    custom_enum2 my_type2 NOT NULL
);
