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

#[derive(Debug, Clone)]
pub struct TableName {
    name: String,
    schema: Option<String>,
}

impl TableName {
    fn new(name: &str, schema: Option<&str>) -> TableName {
        TableName {
          name: name.into(),
          schema: schema.map(String::from),
        }
    }

    pub fn to_string(&self) -> String {
        match self.schema {
            Some(ref schema_name) => format!("{}.{}", schema_name, self.name),
            None => self.name.clone(),
        }
    }
}

mod codegen;
mod inference;

pub use inference::load_table_names;
pub use codegen::*;
