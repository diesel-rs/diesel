use std::ops::Deref;

use quote;

#[derive(Debug, Clone)]
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

    pub fn to_string(&self) -> String {
        match self.schema {
            Some(ref schema_name) => format!("{}.{}", schema_name, self.name),
            None => self.name.clone(),
        }
    }

    pub fn set_tokens(&self, tokens: quote::Tokens) -> TableDataWithTokens {
        TableDataWithTokens {
          table: self.clone(),
          tokens: tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableDataWithTokens {
    /// Table data with name and schema
    table: TableData,
    /// Table represented as tokens of `table!` macro
    tokens: quote::Tokens,
}

impl TableDataWithTokens {
    pub fn tokens(&self) -> quote::Tokens {
        self.tokens.clone()
    }
}

impl Deref for TableDataWithTokens {
    type Target = TableData;

    fn deref(&self) -> &TableData {
        &self.table
    }
}
