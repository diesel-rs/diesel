CREATE EXTENSION IF NOT EXISTS citext;

CREATE SCHEMA custom_schema;

-- user defined types that share their name with a type provided by Diesel
CREATE TYPE public.range AS ENUM ('ideal', 'rated');
CREATE TYPE public.money AS ENUM ('usd', 'eur');
CREATE TYPE custom_schema.interval AS ENUM ('day', 'week');

CREATE TABLE custom_types (
    id SERIAL PRIMARY KEY,
    preferred_range public.range NOT NULL,
    ranges public.range[] NOT NULL,
    currency public.money NOT NULL,
    period custom_schema.interval NOT NULL
);

-- the built-in types are still provided by Diesel
CREATE TABLE builtin_types_1 (
    id SERIAL PRIMARY KEY,
    money_col pg_catalog.money NOT NULL,
    money_array_col pg_catalog.money[] NOT NULL,
    interval_col pg_catalog.interval NOT NULL,
    citext_col citext NOT NULL,
    bool_col bool NOT NULL,
    int2_col int2 NOT NULL,
    int8_col int8 NOT NULL,
    float4_col float4 NOT NULL,
    float8_col float8 NOT NULL,
    numeric_col numeric NOT NULL,
    text_col text NOT NULL,
    varchar_col varchar NOT NULL,
    bpchar_col char(3) NOT NULL,
    char_col "char" NOT NULL,
    bytea_col bytea NOT NULL
);

CREATE TABLE builtin_types_2 (
    id SERIAL PRIMARY KEY,
    date_col date NOT NULL,
    time_col time NOT NULL,
    timestamp_col timestamp NOT NULL,
    timestamptz_col timestamptz NOT NULL,
    uuid_col uuid NOT NULL,
    json_col json NOT NULL,
    jsonb_col jsonb NOT NULL,
    inet_col inet NOT NULL,
    cidr_col cidr NOT NULL,
    macaddr_col macaddr NOT NULL,
    macaddr8_col macaddr8 NOT NULL,
    oid_col oid NOT NULL,
    pg_lsn_col pg_lsn NOT NULL,
    bit_col bit(8) NOT NULL
);

CREATE TABLE builtin_types_3 (
    id SERIAL PRIMARY KEY,
    int4range_col int4range NOT NULL,
    int8range_col int8range NOT NULL,
    daterange_col daterange NOT NULL,
    numrange_col numrange NOT NULL,
    tsrange_col tsrange NOT NULL,
    tstzrange_col tstzrange NOT NULL,
    int4multirange_col int4multirange NOT NULL,
    int8multirange_col int8multirange NOT NULL,
    datemultirange_col datemultirange NOT NULL,
    nummultirange_col nummultirange NOT NULL,
    tsmultirange_col tsmultirange NOT NULL,
    tstzmultirange_col tstzmultirange NOT NULL
);
