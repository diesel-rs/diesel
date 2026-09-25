use super::SchemaResolver;
use super::backend::Backend;
use super::select::SelectField;
use crate::IsNull;
use crate::error::Error;
use crate::error::Result;
use crate::resolver::{CombinedResolver, ResolvedField};
use crate::select::Expression;
use sqlparser::ast::{
    CreateView, ObjectNamePart, Query, SetExpr, TableFactor, TableWithJoins, Visit, Visitor,
    visit_relations,
};
use sqlparser::parser::ParserOptions;
use std::ops::ControlFlow;

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
        let mut resolver = CombinedResolver::new(resolver);
        infer_subqueries(&self.subqueries, &mut resolver)?;
        self.fields
            .iter()
            .map(|f| f.infer_nullability(&mut resolver))
            .collect()
    }

    /// Resolve references to wildcard expressions given the provided schema resolver
    ///
    /// This needs to be called before any other operation is performed with this view definition
    pub fn resolve_references(&mut self, resolver: &mut dyn SchemaResolver) -> Result<()> {
        let mut resolver = CombinedResolver::new(resolver);
        resolve_subquery_wildcards(&mut self.subqueries, &mut resolver)?;
        resolve_wildcards(&mut self.fields, &mut resolver)
    }
}

/// Infer information about a given view definition
///
/// This method accepts both `CREATE VIEW xyz AS SELECT …` and
/// plain `SELECT …` statements as view definition. `backend` is the
/// database the definition comes from, which parses and evaluates it.
pub fn parse_view_def(definition: &str, backend: Backend) -> Result<ViewData> {
    let dialect = backend.dialect();
    let options = ParserOptions::new();

    // `parse_statement` stops wherever the statement seems to end, so anything after
    // that, like the FROM clause of a query it only read in part, would silently be
    // left out of the inference
    let statements = sqlparser::parser::Parser::new(dialect)
        .with_options(options)
        .try_with_sql(definition)?
        .parse_statements()?;
    let [stmt] = <[_; 1]>::try_from(statements).map_err(|statements| Error::UnsupportedSql {
        msg: format!("Expected one statement, found {}", statements.len()),
    })?;

    let select = match stmt {
        sqlparser::ast::Statement::Query(query) => query,
        sqlparser::ast::Statement::CreateView(CreateView { query, .. }) => query,
        stmt => {
            return Err(Error::UnsupportedSql {
                msg: format!("Unexpected statement: `{stmt}`"),
            });
        }
    };
    check_derived_table_aliases(&select)?;
    let subqueries = collect_subqueries(&select, backend)?;
    let results = crate::select::parse_query(&select, None, backend)?;
    Ok(ViewData {
        fields: results,
        subqueries,
    })
}

/// A common table expression or derived table of a view definition
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct SubQuery {
    fields: Vec<SelectField>,
    /// the common table expressions and derived tables of its own query, which only it
    /// sees
    subqueries: Vec<(Option<String>, SubQuery)>,
    /// is it a common table expression of a `WITH RECURSIVE` clause, which can read
    /// itself?
    recursive: bool,
}

/// The common table expressions and derived tables `query` defines itself
///
/// The resolver looks them up by name in the scope of the query, so a query may not
/// define a name twice, and a common table expression may not read one defined after
/// it, which SQLite looks up among the common table expressions and PostgreSQL and MySQL
/// among the tables.
fn collect_subqueries(query: &Query, backend: Backend) -> Result<Vec<(Option<String>, SubQuery)>> {
    let mut subqueries = Vec::new();
    if let Some(with) = &query.with {
        for (index, cte) in with.cte_tables.iter().enumerate() {
            let name = &cte.alias.name.value;
            if let Some(later) = with.cte_tables[index + 1..]
                .iter()
                .find(|later| reads_relation(&cte.query, &later.alias.name.value))
            {
                return Err(Error::UnsupportedSql {
                    msg: format!(
                        "Common table expression `{name}` reads `{}`, which is defined after it",
                        later.alias.name.value
                    ),
                });
            }
            let mut fields = crate::select::parse_query(&cte.query, None, backend)?;
            if !cte.alias.columns.is_empty() {
                if fields.len() == cte.alias.columns.len() {
                    fields
                        .iter_mut()
                        .zip(&cte.alias.columns)
                        .for_each(|(f, a)| {
                            f.ident = Some(a.name.value.clone());
                        });
                } else {
                    return Err(Error::UnsupportedSql {
                        msg: format!(
                            "Not matching field count for a CTE. \
                                 Got {} fields, but expected {} fields",
                            fields.len(),
                            cte.alias.columns.len()
                        ),
                    });
                }
            }
            subqueries.push((
                Some(name.clone()),
                SubQuery {
                    fields,
                    subqueries: collect_subqueries(&cte.query, backend)?,
                    recursive: with.recursive,
                },
            ));
        }
    }
    collect_derived_tables(&query.body, &mut subqueries, backend)?;
    for (index, (name, _)) in subqueries.iter().enumerate() {
        let defined_before = subqueries[..index]
            .iter()
            .any(|(other, _)| match (name, other) {
                (Some(name), Some(other)) => name.eq_ignore_ascii_case(other),
                (None, None) => true,
                _ => false,
            });
        if defined_before {
            return Err(Error::UnsupportedSql {
                msg: format!(
                    "Query defines `{}` twice",
                    name.as_deref().unwrap_or("an unnamed derived table")
                ),
            });
        }
    }
    Ok(subqueries)
}

/// The derived tables in the `FROM` clauses of `body`
fn collect_derived_tables(
    body: &SetExpr,
    subqueries: &mut Vec<(Option<String>, SubQuery)>,
    backend: Backend,
) -> Result<()> {
    match body {
        SetExpr::Select(select) => {
            for table in &select.from {
                collect_derived_table(table, subqueries, backend)?;
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_derived_tables(left, subqueries, backend)?;
            collect_derived_tables(right, subqueries, backend)?;
        }
        _ => {}
    }
    Ok(())
}

fn collect_derived_table(
    table: &TableWithJoins,
    subqueries: &mut Vec<(Option<String>, SubQuery)>,
    backend: Backend,
) -> Result<()> {
    for factor in std::iter::once(&table.relation).chain(table.joins.iter().map(|j| &j.relation)) {
        match factor {
            TableFactor::Derived {
                lateral: false,
                subquery,
                alias,
                sample: None,
            } => subqueries.push((
                alias.as_ref().map(|a| a.name.value.clone()),
                SubQuery {
                    fields: crate::select::parse_query(subquery, None, backend)?,
                    subqueries: collect_subqueries(subquery, backend)?,
                    recursive: false,
                },
            )),
            TableFactor::NestedJoin {
                table_with_joins, ..
            } => collect_derived_table(table_with_joins, subqueries, backend)?,
            _ => {}
        }
    }
    Ok(())
}

/// Does `query` read a relation called `name`, ignoring ASCII case?
fn reads_relation(query: &Query, name: &str) -> bool {
    visit_relations(query, |relation| match relation.0.as_slice() {
        [ObjectNamePart::Identifier(ident)] if ident.value.eq_ignore_ascii_case(name) => {
            ControlFlow::Break(())
        }
        _ => ControlFlow::Continue(()),
    })
    .is_break()
}

/// Reject a definition that reads a relation by a name a derived table has somewhere
///
/// The resolver looks a derived table up by its alias, while a relation of the same name
/// read in another query means a table or common table expression.
fn check_derived_table_aliases(query: &Query) -> Result<()> {
    struct Aliases(Vec<String>);

    impl Visitor for Aliases {
        type Break = ();

        fn pre_visit_table_factor(&mut self, factor: &TableFactor) -> ControlFlow<()> {
            if let TableFactor::Derived {
                alias: Some(alias), ..
            } = factor
            {
                self.0.push(alias.name.value.clone());
            }
            ControlFlow::Continue(())
        }
    }

    let mut aliases = Aliases(Vec::new());
    let _ = query.visit(&mut aliases);
    match aliases.0.iter().find(|alias| reads_relation(query, alias)) {
        Some(alias) => Err(Error::UnsupportedSql {
            msg: format!("Relation `{alias}` is read, but also names a derived table"),
        }),
        None => Ok(()),
    }
}

/// Infer the nullability of the fields of `subqueries`, and bring them into scope
///
/// The relations a query defines itself are only in scope while inferring that query.
fn infer_subqueries(
    subqueries: &[(Option<String>, SubQuery)],
    resolver: &mut CombinedResolver<'_>,
) -> Result<()> {
    for (name, subquery) in subqueries {
        let scope = resolver.enter(name, subquery);
        infer_subqueries(&subquery.subqueries, resolver)?;
        let fields = subquery
            .fields
            .iter()
            .map(|f| {
                Ok(ResolvedField::new(
                    f.ident.clone(),
                    f.infer_nullability(resolver)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        resolver.leave(scope);
        resolver.define(name.clone(), fields);
    }
    Ok(())
}

/// Resolve the wildcards of `subqueries`, and bring them into scope
fn resolve_subquery_wildcards(
    subqueries: &mut [(Option<String>, SubQuery)],
    resolver: &mut CombinedResolver<'_>,
) -> Result<()> {
    for (name, subquery) in subqueries {
        let scope = resolver.enter(name, subquery);
        resolve_subquery_wildcards(&mut subquery.subqueries, resolver)?;
        resolve_wildcards(&mut subquery.fields, resolver)?;
        resolver.leave(scope);
        resolver.define(name.clone(), unknown_fields(&subquery.fields));
    }
    Ok(())
}

/// The fields of a relation whose nullability is not inferred yet
pub(crate) fn unknown_fields(fields: &[SelectField]) -> Vec<ResolvedField> {
    fields
        .iter()
        .map(|f| ResolvedField::new(f.ident.clone(), IsNull::Unknown))
        .collect()
}

impl SubQuery {
    pub(crate) fn fields(&self) -> &[SelectField] {
        &self.fields
    }

    pub(crate) fn recursive(&self) -> bool {
        self.recursive
    }
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
            nullable_row,
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
                        nullable_row: *nullable_row,
                    },
                });
            }
        } else {
            fields.push(f);
        }
    }

    Ok(())
}
