CREATE TABLE special_posts (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL,
  title VARCHAR NOT NULL
);

CREATE TABLE special_comments (
  id SERIAL PRIMARY KEY,
  special_post_id INTEGER NOT NULL
);
