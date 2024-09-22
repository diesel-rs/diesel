CREATE TABLE self_referential_fk (
  id SERIAL PRIMARY KEY,
  parent_id INTEGER NOT NULL
);

ALTER TABLE self_referential_fk ADD CONSTRAINT self_referential_fk_parent_id_fk FOREIGN KEY (parent_id) REFERENCES self_referential_fk;

ALTER TABLE posts ADD CONSTRAINT title_is_unique UNIQUE (title);
CREATE TABLE fk_doesnt_reference_pk (
  id SERIAL PRIMARY KEY,
  random TEXT REFERENCES posts (title)
);

CREATE TABLE composite_fk (
  id SERIAL PRIMARY KEY,
  post_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  FOREIGN KEY (user_id, post_id) REFERENCES followings
);

CREATE TABLE multiple_fks_to_same_table (
  id SERIAL PRIMARY KEY,
  post_id_1 INTEGER REFERENCES posts,
  post_id_2 INTEGER REFERENCES posts
);

CREATE TABLE cyclic_fk_1 (
  id SERIAL PRIMARY KEY,
  cyclic_fk_2_id INTEGER
);

CREATE TABLE cyclic_fk_2 (
  id SERIAL PRIMARY KEY,
  cyclic_fk_1_id INTEGER REFERENCES cyclic_fk_1
);

ALTER TABLE cyclic_fk_1 ADD CONSTRAINT cyclic_fk_1_cyclic_fk_2_id_fk FOREIGN KEY (cyclic_fk_2_id) REFERENCES cyclic_fk_2;
