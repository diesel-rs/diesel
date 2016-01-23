#[cfg(feature = "postgres")] // FIXME: There are valuable tests for SQLite here
mod annotations;
mod deserialization;
mod insert;
mod schema;
#[cfg(feature = "postgres")] // FIXME: There are valuable tests for SQLite here
mod update;
