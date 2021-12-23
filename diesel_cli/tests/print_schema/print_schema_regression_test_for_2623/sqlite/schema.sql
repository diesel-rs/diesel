CREATE TABLE tab_key1(
    id integer NOT NULL,
    PRIMARY KEY (id)
);


CREATE TABLE tab_problem (
  id integer NOT NULL,
  key1 bigint NOT NULL,

  PRIMARY KEY (id),
  UNIQUE (key1),
  CONSTRAINT `key1` FOREIGN KEY (key1) REFERENCES tab_key1(id)
);
