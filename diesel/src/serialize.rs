//! Types and traits related to serializing values for the database

use std::error::Error;
use std::result;

use types::IsNull;

/// A specialized result type representing the result of serializing
/// a value for the database.
pub type Result = result::Result<IsNull, Box<Error + Send + Sync>>;
