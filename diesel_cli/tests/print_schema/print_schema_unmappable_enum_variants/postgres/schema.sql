CREATE TYPE delivery_option AS ENUM ('standard', '24h', 'self', 'mod');

CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    delivery delivery_option NOT NULL
);
