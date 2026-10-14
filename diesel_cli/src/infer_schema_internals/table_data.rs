use std::fmt;

use super::data_structures::ColumnDefinition;
use super::inference;
use crate::config::LeadingDigitPrefixes;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableName {
    pub sql_name: String,
    pub rust_name: String,
    pub schema: Option<String>,
    pub rust_schema: Option<String>,
}

impl TableName {
    pub fn from_name<T: Into<String>>(
        name: T,
        leading_digit_prefixes: &LeadingDigitPrefixes,
    ) -> Self {
        let name = name.into();

        TableName {
            rust_name: inference::rust_name_for_sql_name(
                &name,
                None,
                leading_digit_prefixes.table.as_str(),
            ),
            sql_name: name,
            schema: None,
            rust_schema: None,
        }
    }

    pub fn new<T, U>(name: T, schema: U, leading_digit_prefixes: &LeadingDigitPrefixes) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        let name = name.into();
        let schema = schema.into();
        TableName {
            rust_name: inference::rust_name_for_sql_name(
                &name,
                None,
                leading_digit_prefixes.table.as_str(),
            ),
            sql_name: name,
            rust_schema: Some(inference::rust_name_for_sql_name(
                &schema,
                None,
                inference::SCHEMA_LEADING_DIGIT_PREFIX,
            )),
            schema: Some(schema),
        }
    }

    #[cfg(feature = "uses_information_schema")]
    pub fn strip_schema_if_matches(&mut self, schema: &str) {
        if self.schema.as_deref() == Some(schema) {
            self.schema = None;
            self.rust_schema = None;
        }
    }

    pub fn full_sql_name(&self) -> String {
        match self.schema {
            Some(ref schema_name) => format!("{}.{}", schema_name, self.sql_name),
            None => self.sql_name.to_string(),
        }
    }
}

impl fmt::Display for TableName {
    fn fmt(&self, out: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.rust_schema {
            Some(ref schema_name) => write!(out, "{}.{}", schema_name, self.rust_name),
            None => write!(out, "{}", self.rust_name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableData {
    pub name: TableName,
    pub primary_key: Vec<String>,
    pub column_data: Vec<ColumnDefinition>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ViewData {
    pub name: TableName,
    pub column_data: Vec<ColumnDefinition>,
    pub comment: Option<String>,
    #[expect(dead_code, reason = "Will be used later")]
    pub sql_definition: String,
}
