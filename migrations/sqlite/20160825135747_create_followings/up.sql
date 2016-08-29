CREATE TABLE followings (
  user_id INTEGER NOT NULL,
  post_id INTEGER NOT NULL,
  email_notifications BOOLEAN NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, post_id)
);
