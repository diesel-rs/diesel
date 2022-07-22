CREATE TABLE data_centers (
   id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
   name TEXT NOT NULL
);

CREATE TABLE accounts (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  account TEXT NOT NULL,
  data_center_id INTEGER NOT NULL,
  auth_key BLOB NOT NULL,
  UNIQUE (account, data_center_id),
  FOREIGN KEY (data_center_id) REFERENCES data_centers
);
