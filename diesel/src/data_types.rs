//! Structs to represent the primitive equivalent of SQL types where
//! there is no existing Rust primitive, or where using it would be
//! confusing (such as date and time types). This module will re-export
//! all backend specific data structures when compiled against that
//! backend.
#[cfg(feature = "postgres_backend")]
pub use crate::pg::data_types::*;

#[cfg(feature = "mysql_backend")]
pub use crate::mysql::data_types::*;
