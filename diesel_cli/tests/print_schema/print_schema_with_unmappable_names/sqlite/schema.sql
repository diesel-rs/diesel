CREATE TABLE self (id INTEGER NOT NULL PRIMARY KEY);
CREATE TABLE "user-has::complex>>>role" (
  "user" INTEGER NOT NULL REFERENCES self(id),
  role INTEGER NOT NULL,
  id INTEGER NOT NULL PRIMARY KEY,
  "created at" TIMESTAMP NOT NULL,
  "expiry date" TIMESTAMP
);
