CREATE SCHEMA custom_schema;
CREATE TABLE custom_schema.a(
        id serial NOT NULL,
        CONSTRAINT a_pkey PRIMARY KEY (id)

);
CREATE TABLE custom_schema.b(
        id serial NOT NULL,
        parent serial NOT NULL,
        CONSTRAINT b_pkey PRIMARY KEY (id)

);
ALTER TABLE custom_schema.b ADD CONSTRAINT ab_fkey FOREIGN KEY (parent)
REFERENCES custom_schema.a (id) MATCH FULL
ON DELETE NO ACTION ON UPDATE NO ACTION;
