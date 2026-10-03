use super::SchemaResolver;
use super::select::SelectField;
use crate::IsNull;
use crate::error::Error;
use crate::error::Result;
use crate::resolver::CombinedResolver;
use crate::select::Expression;
use sqlparser::ast::CreateView;
use sqlparser::parser::ParserOptions;

/// An opaque representation of information
/// about a specific SQL view
///
/// Use the provided methods to access the information
#[derive(Debug, PartialEq)]
pub struct ViewData {
    pub(crate) fields: Vec<SelectField>,
    pub(crate) subqueries: Vec<SubQuery>,
}

impl ViewData {
    /// The number of fields returned by this VIEW
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Infer the nullablity of all VIEW fields
    ///
    /// This function returns one `IsNull` per field, in the same number and
    /// order as the fields returned by this VIEW.
    ///
    /// Each value indicates whether the field is definitely nullable
    /// (`IsNull::IsNullable`), definitely not nullable
    /// (`IsNull::NotNullable`), or whether its nullability could not be
    /// determined (`IsNull::Unknown`)
    ///
    /// This method accepts a generic [`SchemaResolver`]
    /// to query information about relations used in this
    /// view definition
    pub fn infer_nullability(&self, resolver: &mut dyn SchemaResolver) -> Result<Vec<IsNull>> {
        let mut resolver = CombinedResolver::new(&self.subqueries, resolver)?;

        self.fields
            .iter()
            .map(|f| f.infer_nullability(&mut resolver))
            .collect()
    }

    /// Resolve references to wildcard expressions given the provided schema resolver
    ///
    /// This needs to be called before any other operation is performed with this view definition
    pub fn resolve_references(&mut self, resolver: &mut dyn SchemaResolver) -> Result<()> {
        let mut resolver = CombinedResolver::empty(self.subqueries.len(), resolver);
        for subquery in &mut self.subqueries {
            resolve_wildcards(&mut subquery.fields, &mut resolver)?;
            resolver.insert(&subquery.fields)?;
        }
        resolve_wildcards(&mut self.fields, &mut resolver)
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
        sqlparser::ast::Statement::CreateView(CreateView { query, .. }) => query,
        stmt => {
            return Err(Error::UnsupportedSql {
                msg: format!("Unexpected statement: `{stmt}`"),
            });
        }
    };
    let mut context = ParseContext::default();
    let fields = crate::select::parse_query(&select, None, &mut context)?;
    Ok(ViewData {
        fields,
        subqueries: context.subqueries,
    })
}

#[derive(Debug, PartialEq)]
pub(crate) struct SubQuery {
    pub(crate) fields: Vec<SelectField>,
}

#[derive(Default)]
pub(crate) struct ParseContext {
    pub(crate) subqueries: Vec<SubQuery>,
    pub(crate) ctes: Vec<(String, usize)>,
}

impl ParseContext {
    pub(crate) fn register(&mut self, fields: Vec<SelectField>) -> usize {
        let id = self.subqueries.len();
        self.subqueries.push(SubQuery { fields });
        id
    }

    pub(crate) fn cte(&self, name: &str) -> Option<usize> {
        self.ctes
            .iter()
            .rev()
            .find_map(|(candidate, id)| (candidate == name).then_some(*id))
    }
}

fn resolve_wildcards(
    fields: &mut Vec<SelectField>,
    resolver: &mut CombinedResolver<'_>,
) -> Result<()> {
    let old_fields = std::mem::take(fields);
    fields.reserve(old_fields.len());
    for f in old_fields {
        if let Expression::Wildcard {
            schema,
            relation,
            is_left_joined,
            source_id,
        } = &f.kind
        {
            let resolved_fields = match source_id {
                Some(id) => resolver.list_derived_fields(*id)?,
                None => resolver
                    .list_fields(schema.as_deref(), relation.as_deref())
                    .map_err(|inner| Error::ResolverFailure { inner })?,
            };
            for f in resolved_fields {
                fields.push(SelectField {
                    ident: f.name().map(|n| n.to_owned()),
                    kind: Expression::Field {
                        schema: schema.clone(),
                        query_source: relation.clone(),
                        field_name: f.name().ok_or(Error::UnnamedField)?.to_owned(),
                        source_id: *source_id,
                        via_left_join: *is_left_joined,
                    },
                });
            }
        } else {
            fields.push(f);
        }
    }

    Ok(())
}
