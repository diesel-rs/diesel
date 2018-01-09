use diesel::*;
use diesel::backend::Backend;
use diesel::deserialize::FromSqlRow;
use std::fmt;
use std::str::FromStr;

use data_structures::ColumnDefinition;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableName {
    pub name: String,
    pub schema: Option<String>,
}

impl TableName {
    pub fn from_name<T: Into<String>>(name: T) -> Self {
        TableName {
            name: name.into(),
            schema: None,
        }
    }

    pub fn new<T, U>(name: T, schema: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        TableName {
            name: name.into(),
            schema: Some(schema.into()),
        }
    }

    pub fn strip_schema_if_matches(&mut self, schema: &str) {
        if self.schema.as_ref().map(|s| &**s) == Some(schema) {
            self.schema = None;
        }
    }
}

impl<ST, DB> Queryable<ST, DB> for TableName
where
    DB: Backend,
    (String, String): FromSqlRow<ST, DB>,
{
    type Row = (String, String);

    fn build((name, schema): Self::Row) -> Self {
        TableName::new(name, schema)
    }
}

impl fmt::Display for TableName {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.schema {
            Some(ref schema_name) => write!(out, "{}.{}", schema_name, self.name),
            None => write!(out, "{}", self.name),
        }
    }
}

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub enum Never {}

impl FromStr for TableName {
    type Err = Never;

    fn from_str(table_name: &str) -> Result<Self, Self::Err> {
        let mut parts = table_name.split('.');
        match (parts.next(), parts.next()) {
            (Some(schema), Some(name)) => Ok(TableName::new(name, schema)),
            _ => Ok(TableName::from_name(table_name)),
        }
    }
}

#[derive(Debug)]
pub struct TableData {
    pub name: TableName,
    pub primary_key: Vec<String>,
    pub column_data: Vec<ColumnDefinition>,
    pub docs: String,
}
