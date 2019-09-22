use diesel::backend::Backend;
use diesel::deserialize::{FromSqlRow, Queryable};
use std::fmt;
use std::str::FromStr;

use super::data_structures::ColumnDefinition;
use super::inference;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableName {
    pub sql_name: String,
    pub schema: Option<String>,
    pub rust_name: Option<String>,
}

impl TableName {
    pub fn from_name<T: Into<String>>(name: T) -> Self {
        let name = name.into();

        TableName {
            sql_name: name.clone(),
            schema: None,
            rust_name: inference::rust_name_for_column(&name),
        }
    }

    pub fn new<T, U>(name: T, schema: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        let name = name.into();

        TableName {
            sql_name: name.clone(),
            schema: Some(schema.into()),
            rust_name: inference::rust_name_for_column(&name),
        }
    }

    #[cfg(feature = "uses_information_schema")]
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
            Some(ref schema_name) => write!(out, "{}.{}", schema_name, self.sql_name),
            None => write!(out, "{}", self.sql_name),
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

mod serde_impls {
    extern crate serde;

    use self::serde::de::Visitor;
    use self::serde::{de, Deserialize, Deserializer};
    use super::TableName;
    use std::fmt;

    impl<'de> Deserialize<'de> for TableName {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct TableNameVisitor;

            impl<'de> Visitor<'de> for TableNameVisitor {
                type Value = TableName;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("A valid table name")
                }

                fn visit_str<E>(self, value: &str) -> Result<TableName, E>
                where
                    E: de::Error,
                {
                    value.parse().map_err(|_| unreachable!())
                }
            }

            deserializer.deserialize_string(TableNameVisitor)
        }
    }
}
