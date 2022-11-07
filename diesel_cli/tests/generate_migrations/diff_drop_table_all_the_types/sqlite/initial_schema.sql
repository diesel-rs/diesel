CREATE TABLE test(
       id INTEGER NOT NULL PRIMARY KEY,
       name TEXT,
       number REAL,
       blob BLOB NOT NULL
);

CREATE TABLE test2(
       id INTEGER NOT NULL PRIMARY KEY,
       bigint BIGINT NOT NULL,
       number UNSIGNED BIG INT NOT NULL,
       text VARCHAR(255),
       clob CLOB,
       other_text NATIVE CHARACTER(70),
       double DOUBLE,
       double2 DOUBLE PRECISION,
       float FLOAT,
       numeric NUMERIC,
       date DATE,
       timestamp TIMESTAMP,
       datetime DATETIME,
       decimal DECIMAL(10,5)
);
