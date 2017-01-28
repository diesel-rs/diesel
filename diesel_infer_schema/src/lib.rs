#[macro_use]
extern crate diesel;
#[macro_use]
extern crate quote;
extern crate syn;

mod data_structures;
#[cfg(feature = "postgres")]
mod pg;
#[cfg(feature = "sqlite")]
mod sqlite;

mod codegen;
mod inference;

pub use codegen::*;
