CREATE TABLE users (id INTEGER PRIMARY KEY NOT NULL, name VARCHAR(50) NOT NULL, bytes VARBINARY(16) NOT NULL);
INSERT INTO users VALUES (1, 'a', x'FF'), (2, '', x'');

-- a division by zero is NULL
CREATE VIEW operators AS SELECT 1 DIV 0 AS quotient, 1 / 0 AS ratio, 'abc' LIKE 'a%' AS starts_with_a, 'abc' REGEXP '^a' AS matches_a, CAST('1' AS SIGNED) AS one;
-- a result longer than max_allowed_packet is NULL
CREATE VIEW casts AS SELECT id, CAST(name AS CHAR) AS name_text, CAST('x' AS CHAR(1025)) AS padded FROM users;
