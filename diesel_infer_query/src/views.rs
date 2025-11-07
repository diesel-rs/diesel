use super::SchemaResolver;
use super::select::SelectField;
use super::select::infer_from_select;
use crate::error::Error;
use crate::error::Result;
use sqlparser::parser::ParserOptions;

/// An opaque representation of information
/// about a specific SQL view
///
/// Use the provided methods to access the information
#[derive(Debug, PartialEq)]
pub struct ViewData {
    pub(crate) fields: Vec<SelectField>,
}

impl ViewData {
    /// The number of fields returned by this VIEW
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Infer the nullablity of all VIEW fields
    ///
    /// This function returns a vector of optional booleans.
    /// The number and order of elements in this vector corresponds to
    /// the number and order of fields returned by this VIEW.
    ///
    /// Each value indicates whether the field is nullable (`Some(true)`),
    /// not nullable (`Some(false)`) or if the nullablity could
    /// not be inferred (`None`)
    ///
    /// This method accepts a generic [`SchemaResolver`]
    /// to query information about relations used in this
    /// view definition
    pub fn infer_nullability(
        &self,
        resolver: &mut dyn SchemaResolver,
    ) -> Result<Vec<Option<bool>>> {
        self.fields
            .iter()
            .map(|f| f.infer_nullability(resolver))
            .collect()
    }
}

/// Infer information about a given view definition
///
/// This method accepts both `CREATE VIEW xyz AS SELECT …` and
/// plain `SELECT …` statements as view definition.
pub fn parse_view_def(definition: &str) -> Result<ViewData> {
    let dialect = sqlparser::dialect::SQLiteDialect {};
    let options = ParserOptions::new();

    let stmt = sqlparser::parser::Parser::new(&dialect)
        .with_options(options)
        .try_with_sql(definition)?
        .parse_statement()?;

    let select = match stmt {
        sqlparser::ast::Statement::Query(query) => query,
        sqlparser::ast::Statement::CreateView { query, .. } => query,
        stmt => {
            return Err(Error::UnsupportedSql {
                msg: format!("Unexpected statement: `{stmt}`"),
            });
        }
    };
    let result = match &*select.body {
        sqlparser::ast::SetExpr::Select(select) => infer_from_select(select),
        // we likely want to support more complex queries here as well (UNION, CTE, etc)
        s => {
            return Err(Error::UnsupportedSql {
                msg: format!("Unsupported query kind: `{s}`"),
            });
        }
    };
    Ok(ViewData { fields: result? })
}
