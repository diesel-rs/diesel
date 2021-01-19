CREATE TABLE tab1(
    id bigserial NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE tab_problem (
  id bigserial NOT NULL,
  key1 bigint NOT NULL,

  PRIMARY KEY (id),
  UNIQUE (key1),
  CONSTRAINT "key1" FOREIGN KEY (key1) REFERENCES tab1(id)
);
