CREATE TABLE self_referential_fk (
  id INTEGER PRIMARY KEY,
  parent_id INTEGER NOT NULL,
  FOREIGN KEY (parent_id) REFERENCES self_referential_fk (id)
);

CREATE UNIQUE INDEX posts_title_is_unique ON posts (title);
CREATE TABLE fk_doesnt_reference_pk (
  id INTEGER PRIMARY KEY,
  random TEXT,
  FOREIGN KEY (random) REFERENCES posts (title)
);

CREATE TABLE composite_fk (
  id INTEGER PRIMARY KEY,
  post_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  FOREIGN KEY (user_id, post_id) REFERENCES followings (user_id, post_id)
);

CREATE TABLE multiple_fks_to_same_table (
  id INTEGER PRIMARY KEY,
  post_id_1,
  post_id_2,
  FOREIGN KEY (post_id_1) REFERENCES posts (id),
  FOREIGN KEY (post_id_2) REFERENCES posts (id)
);

CREATE TABLE cyclic_fk_1 (
  id INTEGER PRIMARY KEY,
  cyclic_fk_2_id,
  FOREIGN KEY (cyclic_fk_2_id) REFERENCES cyclic_fk_2 (id)
);

CREATE TABLE cyclic_fk_2 (
  id INTEGER PRIMARY KEY,
  cyclic_fk_1_id,
  FOREIGN KEY (cyclic_fk_1_id) REFERENCES cyclic_fk_1 (id)
);
