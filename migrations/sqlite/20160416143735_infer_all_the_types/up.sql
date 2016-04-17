-- Semi-exhaustive checking of many possible invocations of supported types
-- listed at https://www.sqlite.org/datatype3.html to ensure it compiles
CREATE TABLE infer_all_the_ints (
  col1 INTEGER PRIMARY KEY NOT NULL,
  col2 INT NOT NULL,
  col3 INTEGER NOT NULL,
  col4 LOL_WHAT_EVEN_IS_THIS_TYPE_CAN_I_HAVE_A_HINT NOT NULL,
  col5 SMALLINT NOT NULL,
  col6 SMALLINT(2) NOT NULL,
  col7 SMALL INT NOT NULL,
  col8 BIGINT NOT NULL,
  col9 BIGINT(4) NOT NULL,
  col10 BIG INT NOT NULL,
  col11 INT2 NOT NULL,
  col12 INT4 NOT NULL,
  col13 INT8 NOT NULL
);

CREATE TABLE infer_all_the_bools (
  col1 TINYINT(1) PRIMARY KEY NOT NULL,
  col2 TINYINT NOT NULL,
  col3 TINY INT NOT NULL,
  col4 BOOLEAN NOT NULL
);

CREATE TABLE infer_all_the_strings (
  col1 CHARACTER(20) PRIMARY KEY NOT NULL,
  col2 VARCHAR(255) NOT NULL,
  col3 VARYING CHARACTER(255) NOT NULL,
  col4 NCHAR(55) NOT NULL,
  col5 NATIVE CHARACTER(70) NOT NULL,
  col6 NVARCHAR(100) NOT NULL,
  col7 TEXT NOT NULL,
  col8 CLOB NOT NULL,
  col9 BLOB NOT NULL,
  col10 NOT NULL
);

CREATE TABLE infer_all_the_floats (
  col1 REAL PRIMARY KEY NOT NULL,
  col2 FLOAT NOT NULL,
  col3 DOUBLE NOT NULL,
  col4 DOUBLE PRECISION NOT NULL,
  col5 NUMERIC NOT NULL,
  col6 DECIMAL(10, 5) NOT NULL
)
