CREATE TABLE `test"
include!(env!("P1"))` (
  `id` int(11) NOT NULL PRIMARY KEY,
  `x"] }
  include!(env!("P1")); mod y { #[sql_name="z` enum('active " include!(env!("P1"))','disabled') NOT NULL DEFAULT 'disabled',
  `x"] }
  include!(env!("P")); mod y { #[sql_name="z` set('val1','val2','val3','val4') NOT NULL DEFAULT 'val1'
);

CREATE TABLE test2(
    id INTEGER NOT NULL PRIMARY KEY,
    `column."
include!(env!("COL"));` INTEGER,
CONSTRAINT fk_test2 FOREIGN KEY (`column."
include!(env!("COL"));`) REFERENCES `test"
include!(env!("P1"))`(id));

CREATE TABLE test3(`pk."
include!(env!("PK"));` INTEGER NOT NULL PRIMARY KEY);

CREATE TABLE test(id INTEGER NOT NULL PRIMARY KEY, other INTEGER,
CONSTRAINT fk_test FOREIGN KEY (other) REFERENCES test3 (`pk."
include!(env!("PK"));`));
