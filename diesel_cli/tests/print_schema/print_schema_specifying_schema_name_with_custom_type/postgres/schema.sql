CREATE SCHEMA custom_schema;
CREATE TABLE in_public (id SERIAL PRIMARY KEY);
CREATE TYPE my_public_enum AS ENUM('A', 'B');
CREATE TYPE custom_schema.my_enum AS ENUM ('A', 'B');
CREATE TABLE custom_schema.in_schema (id SERIAL PRIMARY KEY, custom_type custom_schema.MY_ENUM);
