CREATE TYPE commented_enum AS ENUM ('first', 'second');
COMMENT ON TYPE commented_enum IS 'An enum with a comment
  - first: the first label

  - second: the second label';
CREATE TYPE uncommented_enum AS ENUM ('third');
CREATE TABLE with_custom_types (
    id INTEGER PRIMARY KEY,
    commented commented_enum NOT NULL,
    uncommented uncommented_enum NOT NULL
);
