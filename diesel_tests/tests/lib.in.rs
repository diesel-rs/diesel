#[cfg(not(feature = "sqlite"))]
mod annotations;
mod deserialization;
mod insert;
mod schema;
mod schema_inference;
mod update;
