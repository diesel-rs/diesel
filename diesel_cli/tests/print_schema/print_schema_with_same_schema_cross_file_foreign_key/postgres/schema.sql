CREATE SCHEMA inventory;

CREATE TABLE inventory.customers (
    id SERIAL PRIMARY KEY
);

CREATE TABLE inventory.orders (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES inventory.customers(id)
);
