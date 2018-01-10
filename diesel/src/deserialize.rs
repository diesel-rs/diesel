//! Types and traits related to deserializing values from the database

use std::error::Error;
use std::result;

/// A specialized result type representing the result of deserializing
/// a value from the database.
pub type Result<T> = result::Result<T, Box<Error + Send + Sync>>;
