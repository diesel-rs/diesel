CREATE TABLE stuff (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  important BOOLEAN NOT NULL DEFAULT 'f'
);

CREATE VIEW important_stuff AS
  SELECT id, name
  FROM stuff
  WHERE important = 't';
