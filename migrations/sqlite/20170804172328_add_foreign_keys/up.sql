-- SQLite has no useful ALTER TABLE statement for this, so we must drop and
-- re-create them. Table definitions came from `SELECT sql FROM sqlite_master`

DROP TABLE posts;
CREATE TABLE posts (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  user_id INTEGER NOT NULL,
  title VARCHAR NOT NULL,
  body TEXT,
  FOREIGN KEY (user_id) REFERENCES users (id)
);
CREATE INDEX posts_user_id ON posts (user_id);

DROP TABLE comments;
CREATE TABLE comments (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  post_id INTEGER NOT NULL,
  text TEXT NOT NULL,
  FOREIGN KEY (post_id) REFERENCES posts (id)
);

DROP TABLE followings;
CREATE TABLE followings (
  user_id INTEGER NOT NULL,
  post_id INTEGER NOT NULL,
  email_notifications BOOLEAN NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, post_id),
  FOREIGN KEY (user_id) REFERENCES users (id),
  FOREIGN KEY (post_id) REFERENCES posts (id)
);

DROP TABLE likes;
CREATE TABLE likes (
  comment_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  PRIMARY KEY (comment_id, user_id),
  FOREIGN KEY (user_id) REFERENCES users (id),
  FOREIGN KEY (comment_id) REFERENCES comments (id)
);
