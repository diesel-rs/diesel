-- Your SQL goes here
CREATE TABLE company (
  company_id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_code TEXT NOT NULL,
  company_name TEXT NOT NULL,
  address TEXT,
  date_created TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP  
);
