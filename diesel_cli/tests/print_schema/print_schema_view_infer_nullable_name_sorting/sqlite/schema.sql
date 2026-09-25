CREATE TABLE users (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    hair_color TEXT
);
INSERT INTO users VALUES (1, 'n', NULL);

CREATE VIEW reordered AS
SELECT users.name AS z, users.hair_color AS a
FROM users;

CREATE TABLE left_items (
    z_left TEXT NOT NULL,
    a_left TEXT,
    left_id INTEGER PRIMARY KEY NOT NULL
);

CREATE TABLE right_items (
    y_right TEXT,
    b_right TEXT NOT NULL,
    right_id INTEGER PRIMARY KEY NOT NULL
);

CREATE VIEW joined AS
SELECT left_items.*, right_items.*
FROM left_items
INNER JOIN right_items ON left_items.left_id = right_items.right_id;
