CREATE TYPE "some_enum" AS enum('a', 'b', 'c');
CREATE TYPE "some_enum2" AS enum('FOOBAR', 'BAZBOOM');
CREATE TABLE "resource"(
	"resource_id" INT4 NOT NULL PRIMARY KEY,
	"some_field" some_enum NOT NULL,
	"some_field2" some_enum2 NOT NULL
);
