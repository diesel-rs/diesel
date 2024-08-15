-- Your SQL goes here
CREATE SCHEMA IF NOT EXISTS smdb;

CREATE TYPE protocol_type AS ENUM (
    'UnknownProtocol',
    'GRPC',
    'HTTP',
    'UDP'
);

CREATE TYPE service_endpoint AS (
	"name" Text,
	"version" INTEGER,
	"base_uri" Text,
	"port" INTEGER,
	"protocol" protocol_type
);

CREATE TABLE  smdb.service(
	"service_id" INTEGER NOT NULL PRIMARY KEY,
	"name" Text NOT NULL,
	"version" INTEGER NOT NULL,
	"online" BOOLEAN NOT NULL,
	"description" Text NOT NULL,
	"health_check_uri" Text NOT NULL,
	"base_uri" Text NOT NULL,
	"dependencies" INTEGER[] NOT NULL,
	"endpoints" service_endpoint[] NOT NULL
);