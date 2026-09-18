use super::SchemaResolver;
use super::select::SelectField;
use crate::IsNull;
use crate::error::Error;
use crate::error::Result;
use crate::resolver::CombinedResolver;
use crate::select::Expression;
use sqlparser::ast::{CreateView, Query};
use sqlparser::parser::ParserOptions;

/// An opaque representation of information
/// about a specific SQL view
///
/// Use the provided methods to access the information
#[derive(Debug, PartialEq)]
pub struct ViewData {
    pub(crate) fields: Vec<SelectField>,
    pub(crate) subqueries: Vec<(Option<String>, SubQuery)>,
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
        for (_, subquery) in &mut self.subqueries {
            resolve_wildcards(&mut subquery.fields, resolver)?;
        }
        let mut resolver = CombinedResolver::new(&self.subqueries, resolver)?;
        resolve_wildcards(&mut self.fields, &mut resolver)?;
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
        sqlparser::ast::Statement::CreateView(CreateView { query, .. }) => query,
        stmt => {
            return Err(Error::UnsupportedSql {
                msg: format!("Unexpected statement: `{stmt}`"),
            });
        }
    };
    let subqueries = collect_subqueries(&select)?;
    let results = crate::select::parse_query(&select, None)?;
    Ok(ViewData {
        fields: results,
        subqueries,
    })
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct SubQuery {
    fields: Vec<SelectField>,
}

impl SubQuery {
    pub(crate) fn fields(&self) -> &[SelectField] {
        &self.fields
    }
}

fn collect_subqueries(query: &Query) -> Result<Vec<(Option<String>, SubQuery)>> {
    let mut subqueries = if let Some(with) = &query.with {
        with.cte_tables
            .iter()
            .flat_map(|t| match extract_cte_subqueries(t) {
                Ok(o) => Box::new(o.into_iter().map(Ok)) as Box<dyn Iterator<Item = _>>,
                Err(e) => Box::new(std::iter::once(Err(e))),
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    if let sqlparser::ast::SetExpr::Select(select_expr) = &*query.body {
        for s in &select_expr.from {
            extract_subqueries_from_table_factor(&s.relation, &mut subqueries)?;
            for join in &s.joins {
                extract_subqueries_from_table_factor(&join.relation, &mut subqueries)?;
            }
        }
    }

    Ok(subqueries)
}

fn extract_cte_subqueries(
    t: &sqlparser::ast::Cte,
) -> Result<Vec<(Option<String>, SubQuery)>, Error> {
    let name = t.alias.name.value.as_str();
    let mut subqueries = collect_subqueries(&t.query)?;

    let mut fields = crate::select::parse_query(&t.query, None)?;
    if !t.alias.columns.is_empty() {
        if fields.len() == t.alias.columns.len() {
            fields.iter_mut().zip(&t.alias.columns).for_each(|(f, a)| {
                f.ident = Some(a.name.value.clone());
            });
        } else {
            return Err(Error::UnsupportedSql {
                msg: format!(
                    "Not matching field count for a CTE. \
                                 Got {} fields, but expected {} fields",
                    fields.len(),
                    t.alias.columns.len()
                ),
            });
        }
    }

    subqueries.push((Some(name.to_owned()), SubQuery { fields }));
    Ok(subqueries)
}

fn extract_subqueries_from_table_factor(
    s: &sqlparser::ast::TableFactor,
    subqueries: &mut Vec<(Option<String>, SubQuery)>,
) -> Result<()> {
    if let sqlparser::ast::TableFactor::Derived {
        lateral: false,
        subquery,
        alias,
        sample: None,
    } = s
    {
        let fields = crate::select::parse_query(subquery, None)?;
        subqueries.push((
            alias.as_ref().map(|a| a.name.value.clone()),
            SubQuery { fields },
        ));
    }

    Ok(())
}

fn resolve_wildcards(
    fields: &mut Vec<SelectField>,
    resolver: &mut dyn SchemaResolver,
) -> Result<()> {
    let old_fields = std::mem::take(fields);
    fields.reserve(fields.len());
    for f in old_fields {
        if let Expression::Wildcard {
            schema,
            relation,
            is_left_joined,
        } = &f.kind
        {
            let resolved_fields = resolver
                .list_fields(schema.as_deref(), relation.as_deref())
                .map_err(|e| Error::ResolverFailure { inner: e })?;
            for f in resolved_fields {
                fields.push(SelectField {
                    ident: f.name().map(|n| n.to_owned()),
                    kind: Expression::Field {
                        schema: schema.clone(),
                        query_source: relation.clone(),
                        field_name: f.name().ok_or(Error::UnnamedField)?.to_owned(),
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
