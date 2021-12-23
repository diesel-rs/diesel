CREATE SCHEMA custom_schema;
CREATE TABLE custom_schema.self (id SERIAL PRIMARY KEY);
CREATE TABLE custom_schema."user-has::complex>>>role" (
  "user" INTEGER NOT NULL REFERENCES custom_schema.self,
  role INTEGER NOT NULL,
  id SERIAL PRIMARY KEY,
  "created at" TIMESTAMP NOT NULL,
  "expiry date" TIMESTAMP
);
