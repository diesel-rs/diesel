CREATE TYPE some_enum AS enum (
	'a',
	'b',
	'c',
	'd',
	'e',
	'f',
	'g',
	'h'
);

CREATE TYPE some_enum_2 AS enum (
	'a',
	'b',
	'c',
	'd'
);

CREATE TABLE resource (
	resource_id integer NOT NULL PRIMARY KEY,
	some_field some_enum DEFAULT 'e' ::some_enum NOT NULL,
	some_field_2 some_enum_2 NOT NULL
);