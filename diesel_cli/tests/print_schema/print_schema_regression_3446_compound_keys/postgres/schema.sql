CREATE TABLE person (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE payment_card (
    id SERIAL PRIMARY KEY,
    code TEXT NOT NULL,
    holder_id INT NOT NULL REFERENCES person(id),
    UNIQUE (id, code)
);

CREATE TABLE transaction (
    id SERIAL PRIMARY KEY,
    executed_at TIMESTAMPTZ NOT NULL,
    payment_card_id INT NOT NULL,
    card_code TEXT NOT NULL,
    FOREIGN KEY (payment_card_id, card_code) REFERENCES payment_card (id, code)
);
