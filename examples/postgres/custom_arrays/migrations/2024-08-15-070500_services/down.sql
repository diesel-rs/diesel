-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS smdb.service;
DROP TYPE IF EXISTS service_endpoint CASCADE;
DROP TYPE IF EXISTS protocol_type CASCADE;
DROP schema IF EXISTS smdb;