CREATE TABLE users (id INTEGER PRIMARY KEY NOT NULL, name TEXT NOT NULL, doc JSONB NOT NULL);
INSERT INTO users VALUES (1, 'a', '{"k": 1}'), (2, '', 'null');

-- PostgreSQL turns any value into text, but a JSON null into a NULL number
CREATE VIEW casts AS SELECT id::text AS id_text, name::varchar(1) AS name_short, doc::text AS doc_text, (doc -> 'k')::integer AS k FROM users;
-- PostgreSQL's pattern matching is NULL only for a NULL operand
CREATE VIEW patterns AS SELECT name LIKE 'a%' AS starts_with_a, name ILIKE 'A%' AS starts_with_a_ci, name ~ '^a' AS matches_a, name SIMILAR TO 'a%' AS similar_a FROM users;
-- a missing key is NULL
CREATE VIEW operators AS SELECT doc ->> 'k' AS k_text, id IS DISTINCT FROM 1 AS not_first, name || 'x' AS name_x FROM users;
-- an aggregate query without GROUP BY returns a row even without input
CREATE VIEW aggregates AS SELECT count(*) AS n, max(id) AS max_id FROM users WHERE id < 0;
