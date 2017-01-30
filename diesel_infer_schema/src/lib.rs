#[macro_use]
extern crate quote;
extern crate syn;

#[macro_use]
extern crate diesel;

mod codegen;
mod data_structures;
mod inference;
pub mod table_data;

#[cfg(feature = "postgres")]
mod pg;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use codegen::*;
pub use inference::load_table_names;
