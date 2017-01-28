use quote;

#[derive(Debug, Clone)]
pub struct TableData {
    /// Table name
    name: String,
    /// Schema name
    schema: Option<String>,
    /// Table represented as tokens of `table!` macro
    tokens: Option<quote::Tokens>,
}

impl TableData {
    pub fn new(name: &str, schema: Option<&str>) -> TableData {
        TableData {
          name: name.into(),
          schema: schema.map(String::from),
          tokens: None,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn tokens(&self) -> Option<quote::Tokens> {
        self.tokens.clone()
    }

    pub fn set_tokens(&self, tokens: quote::Tokens) -> Self {
        let mut res = self.clone();
        res.tokens = Some(tokens);
        res
    }

    pub fn to_string(&self) -> String {
        match self.schema {
            Some(ref schema_name) => format!("{}.{}", schema_name, self.name),
            None => self.name.clone(),
        }
    }
}
