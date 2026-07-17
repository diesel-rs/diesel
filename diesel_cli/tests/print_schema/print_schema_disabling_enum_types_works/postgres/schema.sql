CREATE TYPE my_type AS ENUM ('foo', 'bar');
CREATE TABLE custom_types (
    id SERIAL PRIMARY KEY,
    custom_enum my_type NOT NULL
);
