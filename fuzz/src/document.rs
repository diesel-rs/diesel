//! A JSON value the fuzzer builds from raw bytes, for the round trip target.

use arbitrary::{Arbitrary, Unstructured};
use serde_json::{Map, Number, Value};

/// Deepest container nesting a generated document reaches.
/// `serde_json`'s text parser reads at most 127 nested containers, while its writer bounds
/// none, so a deeper document is text diesel can write and no `serde_json` reader, diesel's
/// `FromSql<Json>` included, will ever accept.
pub const MAX_NESTING: usize = 127;

/// Widest array or object a generated container holds.
const MAX_CHILDREN: usize = 8;

#[derive(Debug)]
pub enum Document {
    Null,
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    Float(f64),
    Text(String),
    Array(Vec<Document>),
    Object(Vec<(String, Document)>),
}

impl<'a> Arbitrary<'a> for Document {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Self::with_nesting(u, MAX_NESTING)
    }
}

impl Document {
    /// Draws a document within `budget` nested containers, spending one per level.
    /// Once the budget runs out only leaves remain, so every input stays readable.
    fn with_nesting(u: &mut Unstructured<'_>, budget: usize) -> arbitrary::Result<Self> {
        const LEAVES: usize = 6;
        const VARIANTS: usize = 8;
        let variants = if budget == 0 { LEAVES } else { VARIANTS };
        Ok(match u.choose_index(variants)? {
            0 => Document::Null,
            1 => Document::Bool(u.arbitrary()?),
            2 => Document::Signed(u.arbitrary()?),
            3 => Document::Unsigned(u.arbitrary()?),
            4 => Document::Float(u.arbitrary()?),
            5 => Document::Text(u.arbitrary()?),
            6 => {
                let mut items = Vec::new();
                for _ in 0..u.int_in_range(0..=MAX_CHILDREN)? {
                    items.push(Self::with_nesting(u, budget - 1)?);
                }
                Document::Array(items)
            }
            _ => {
                let mut entries = Vec::new();
                for _ in 0..u.int_in_range(0..=MAX_CHILDREN)? {
                    entries.push((u.arbitrary()?, Self::with_nesting(u, budget - 1)?));
                }
                Document::Object(entries)
            }
        })
    }

    /// Container levels deep this document nests.
    pub fn nesting(&self) -> usize {
        match self {
            Document::Array(items) => 1 + items.iter().map(Self::nesting).max().unwrap_or(0),
            Document::Object(entries) => {
                1 + entries
                    .iter()
                    .map(|(_, value)| Self::nesting(value))
                    .max()
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }
}

impl From<&Document> for Value {
    fn from(document: &Document) -> Self {
        match document {
            Document::Null => Value::Null,
            Document::Bool(value) => Value::Bool(*value),
            Document::Signed(value) => Value::Number(Number::from(*value)),
            Document::Unsigned(value) => Value::Number(Number::from(*value)),
            Document::Float(value) => Number::from_f64(*value).map_or(Value::Null, Value::Number),
            Document::Text(value) => Value::String(value.clone()),
            Document::Array(items) => Value::Array(items.iter().map(Value::from).collect()),
            Document::Object(entries) => Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::from(value)))
                    .collect::<Map<_, _>>(),
            ),
        }
    }
}
