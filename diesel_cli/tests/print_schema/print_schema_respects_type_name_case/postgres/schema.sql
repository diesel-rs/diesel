CREATE TYPE "MyType" AS ENUM ('foo', 'bar');
CREATE TYPE "MyType2" AS ENUM ('foo', 'bar');
CREATE TABLE custom_types (
    id SERIAL PRIMARY KEY,
    custom_enum "MyType" NOT NULL,
    custom_enum_nullable "MyType",
    custom_enum2 "MyType2" NOT NULL
);
