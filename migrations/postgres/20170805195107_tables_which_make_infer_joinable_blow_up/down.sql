DROP TABLE self_referential_fk;
DROP TABLE fk_doesnt_reference_pk;
ALTER TABLE posts DROP CONSTRAINT title_is_unique;
DROP TABLE composite_fk;
DROP TABLE multiple_fks_to_same_table;
DROP TABLE cyclic_fk_2 CASCADE;
DROP TABLE cyclic_fk_1 CASCADE;
