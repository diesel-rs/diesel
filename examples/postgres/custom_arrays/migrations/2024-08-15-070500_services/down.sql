-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS smdb.service;
DROP TYPE IF EXISTS smdb.service_endpoint CASCADE;
DROP TYPE IF EXISTS smdb.protocol_type CASCADE;
DROP SCHEMA IF EXISTS smdb;