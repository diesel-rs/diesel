use diesel::backend::Backend;
use diesel::deserialize::{self, FromStaticSqlRow, Queryable};
use std::fmt;
use std::str::FromStr;

use super::data_structures::ColumnDefinition;
use super::inference;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableName {
    pub sql_name: String,
    pub rust_name: String,
    pub schema: Option<String>,
}

impl TableName {
    pub fn from_name<T: Into<String>>(name: T) -> Self {
        let name = name.into();

        TableName {
            rust_name: inference::rust_name_for_sql_name(&name),
            sql_name: name,
            schema: None,
        }
    }

    pub fn new<T, U>(name: T, schema: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        let name = name.into();

        TableName {
            rust_name: inference::rust_name_for_sql_name(&name),
            sql_name: name,
            schema: Some(schema.into()),
        }
    }

    #[cfg(feature = "uses_information_schema")]
    pub fn strip_schema_if_matches(&mut self, schema: &str) {
        if self.schema.as_deref() == Some(schema) {
            self.schema = None;
        }
    }

    pub fn full_sql_name(&self) -> String {
        match self.schema {
            Some(ref schema_name) => format!("{}.{}", schema_name, self.sql_name),
            None => self.sql_name.to_string(),
        }
    }
}

impl<ST, DB> Queryable<ST, DB> for TableName
where
    (String, String): FromStaticSqlRow<ST, DB>,
    DB: Backend,
{
    type Row = (String, String);

    fn build((name, schema): Self::Row) -> deserialize::Result<Self> {
        Ok(TableName::new(name, schema))
    }
}

impl fmt::Display for TableName {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.schema {
            Some(ref schema_name) => write!(out, "{}.{}", schema_name, self.rust_name),
            None => write!(out, "{}", self.rust_name),
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
    pub comment: Option<String>,
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
