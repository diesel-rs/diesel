CREATE TABLE customers (
    id SERIAL PRIMARY KEY
);

CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES customers(id)
);
