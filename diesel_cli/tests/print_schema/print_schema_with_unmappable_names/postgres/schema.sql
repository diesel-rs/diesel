CREATE TABLE self (id SERIAL PRIMARY KEY);
CREATE TABLE "user-has::complex>>>role" (
  "user" INTEGER NOT NULL REFERENCES self,
  role INTEGER NOT NULL,
  id SERIAL PRIMARY KEY,
  "created at" TIMESTAMP NOT NULL,
  "expiry date" TIMESTAMP
);
