#![deny(warnings)]

#[macro_use]
extern crate quote;
extern crate syn;

#[macro_use]
extern crate diesel;

mod codegen;
mod data_structures;
mod inference;
mod table_data;

#[cfg(any(feature = "mysql", feature = "postgres"))]
mod information_schema;
#[cfg(feature = "sqlite")]
mod sqlite;

pub use codegen::*;
pub use inference::load_table_names;
pub use table_data::TableData;
