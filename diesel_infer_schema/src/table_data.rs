use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct TableData {
    /// Table name
    name: String,
    /// Schema name
    schema: Option<String>,
}

impl TableData {
    pub fn new(name: &str, schema: Option<&str>) -> TableData {
        TableData {
          name: name.into(),
          schema: schema.map(String::from),
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn schema(&self) -> &Option<String> {
        &self.schema
    }
}

impl fmt::Display for TableData {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.schema {
            Some(ref schema_name) => write!(out, "{}.{}", schema_name, self.name),
            None => write!(out, "{}", self.name)
        }
    }
}

#[derive(Debug)]
pub enum Never {}
impl FromStr for TableData {
    type Err = Never;

    fn from_str(table_name: &str) -> Result<Self, Self::Err> {
        let mut parts = table_name.split('.');
        match (parts.next(), parts.next()) {
            (Some(schema_name), Some(table_name)) => Ok(TableData::new(table_name, Some(schema_name))),
            _ => Ok(TableData::new(table_name, None)),
        }
    }
}
