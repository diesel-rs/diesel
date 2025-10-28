use super::SchemaResolver;
use super::select::SelectField;
use super::select::infer_from_select;
use crate::error::Error;
use crate::error::Result;
use crate::select::Expression;
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

    /// Resolve references to wildcard expressions given the provided schema resolver
    ///
    /// This needs to be called before any other operation is performed with this view definition
    pub fn resolve_references(&mut self, resolver: &mut dyn SchemaResolver) -> Result<()> {
        let fields = std::mem::take(&mut self.fields);
        self.fields.reserve(fields.len());
        for f in fields {
            if let Expression::Wildcard {
                schema,
                relation,
                is_left_joined,
            } = &f.kind
            {
                let resolved_fields = resolver
                    .list_fields(schema.as_deref(), relation)
                    .map_err(|e| Error::ResolverFailure { inner: e })?;
                for f in resolved_fields {
                    self.fields.push(SelectField {
                        ident: None,
                        kind: Expression::Field {
                            schema: schema.clone(),
                            query_source: relation.clone(),
                            field_name: f.name().to_owned(),
                            via_left_join: *is_left_joined,
                        },
                    });
                }
            } else {
                self.fields.push(f);
            }
        }
        Ok(())
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
