CREATE TABLE likes (
  comment_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  PRIMARY KEY (comment_id, user_id)
);
