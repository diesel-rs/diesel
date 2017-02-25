use diesel::*;
use diesel::backend::Backend;
use diesel::types::{FromSqlRow, HasSqlType};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct TableData {
    pub name: String,
    pub schema: Option<String>,
}

impl TableData {
    pub fn from_name<T: Into<String>>(name: T) -> Self {
        TableData {
            name: name.into(),
            schema: None,
        }
    }

    pub fn new<T, U>(name: T, schema: U) -> Self where
        T: Into<String>,
        U: Into<String>,
    {
        TableData {
            name: name.into(),
            schema: Some(schema.into()),
        }
    }
}

impl<ST, DB> Queryable<ST, DB> for TableData where
    DB: Backend + HasSqlType<ST>,
    Hlist!(String, String): FromSqlRow<ST, DB>,
{
    type Row = Hlist!(String, String);

    fn build(hlist_pat!(name, schema): Self::Row) -> Self {
        TableData::new(name, schema)
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
#[allow(missing_copy_implementations)]
pub enum Never {}

impl FromStr for TableData {
    type Err = Never;

    fn from_str(table_name: &str) -> Result<Self, Self::Err> {
        let mut parts = table_name.split('.');
        match (parts.next(), parts.next()) {
            (Some(schema), Some(name)) => Ok(TableData::new(name, schema)),
            _ => Ok(TableData::from_name(table_name)),
        }
    }
}
