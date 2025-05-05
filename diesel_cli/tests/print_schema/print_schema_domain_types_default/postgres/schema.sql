-- https://www.postgresql.org/docs/current/domains.html
CREATE DOMAIN posint AS integer CHECK (VALUE > 0);

CREATE TABLE mytable (id posint PRIMARY KEY);
