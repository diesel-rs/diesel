CREATE TABLE tab_key1(
    id bigint NOT NULL auto_increment,
    PRIMARY KEY (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;


CREATE TABLE tab_problem (
  id bigint NOT NULL auto_increment,
  key1 bigint NOT NULL,

  PRIMARY KEY (id),
  UNIQUE (key1),
  CONSTRAINT `key1` FOREIGN KEY (key1) REFERENCES tab_key1(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8;
