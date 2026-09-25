use super::SchemaResolver;
use super::backend::Backend;
use super::select::SelectField;
use crate::IsNull;
use crate::error::Error;
use crate::error::Result;
use crate::resolver::{CombinedResolver, ResolvedField};
use crate::select::Expression;
use sqlparser::ast::{
    ArrayElemTypeDef, CreateView, DataType, Expr, Function, FunctionArgumentClause,
    FunctionArguments, JsonTableColumn, ObjectNamePart, Query, SetExpr, Statement, TableAlias,
    TableFactor, TableWithJoins, TypedString, Visit, Visitor, XmlTableColumnOption,
    visit_relations,
};
use sqlparser::parser::{ParserError, ParserOptions};
use sqlparser::tokenizer::{Token, Tokenizer};
use std::ops::ControlFlow;

/// The most tokens, besides whitespace and comments, a definition may have
///
/// sqlparser builds a chain of operators, of `UNION`s, of array brackets like `INT[][]`,
/// or of `PIVOT`s as a tree nested as deep as the chain is long, without its recursion
/// limit applying, and dropping that tree recurses as deep. Limiting the tokens limits
/// that depth: parsing and dropping the longest chains within this limit took at most
/// 796 KiB of stack in an unoptimized and 400 KiB in an optimized build (rustc
/// 1.100.0-nightly, x86_64 Linux).
const MAX_TOKENS: usize = 12_288;

/// The deepest nesting of expressions inference supports
///
/// Inference recurses into operands, and a chain of operators nests as deep as it is
/// long, so deeper nesting could overflow the stack. Lowering an expression took about
/// 19 KiB of stack a level in an unoptimized build (rustc 1.100.0-nightly, x86_64 Linux).
const MAX_EXPRESSION_DEPTH: usize = 64;

/// The most set operations, like `UNION`, a definition may have
///
/// sqlparser builds a chain of set operations as a tree nested as deep as the chain is
/// long, and inference recurses through it.
const MAX_SET_OPERATIONS: usize = 64;

/// The deepest nesting of array types, like the two of `INT[][]`, a definition may have
///
/// sqlparser nests the brackets of an array type as deep as they are many, and lowering
/// a cast or printing an error message recurses as deep. PostgreSQL, the only backend
/// with array types, stores every one with a single pair of brackets.
const MAX_ARRAY_DEPTH: usize = 16;

/// The deepest nesting of query sources, like a table in a `PIVOT` in a `PIVOT` or the
/// nested columns of a `JSON_TABLE`, a definition may have
///
/// sqlparser nests a chain of `PIVOT`s and `UNPIVOT`s as deep as it is long, and the
/// nested columns of a `JSON_TABLE` as deep as they nest, without its recursion limit
/// applying, and printing them in an error message recurses as deep. Other nested query
/// sources, like derived tables and joins in parentheses, stay within that limit.
const MAX_QUERY_SOURCE_DEPTH: usize = 64;

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

    let tokens = Tokenizer::new(dialect, definition)
        .with_unescape(options.unescape)
        .tokenize_with_location()
        .map_err(ParserError::from)?;
    let significant = tokens
        .iter()
        .filter(|token| !matches!(token.token, Token::Whitespace(_)))
        .count();
    if significant > MAX_TOKENS {
        return Err(Error::UnsupportedSql {
            msg: format!("Definitions of more than {MAX_TOKENS} tokens"),
        });
    }
    // `parse_statement` stops wherever the statement seems to end, so anything after
    // that, like the FROM clause of a query it only read in part, would silently be
    // left out of the inference
    let statements = sqlparser::parser::Parser::new(dialect)
        .with_options(options)
        .with_tokens_with_locations(tokens)
        .parse_statements()?;
    let [stmt] = <[_; 1]>::try_from(statements).map_err(|statements| Error::UnsupportedSql {
        msg: format!("Expected one statement, found {}", statements.len()),
    })?;
    let select = match stmt {
        Statement::Query(query) => query,
        Statement::CreateView(CreateView { query, .. }) => query,
        // printing any other statement could recurse into whatever it nests
        _ => {
            return Err(Error::UnsupportedSql {
                msg: String::from("Expected a query or a `CREATE VIEW` statement"),
            });
        }
    };
    // before anything else recurses into the query, including error messages
    if let ControlFlow::Break(msg) = select.visit(&mut Depth::default()) {
        return Err(Error::UnsupportedSql { msg });
    }
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

/// Tracks how deep the visited expressions and query sources nest, how many set
/// operations the visited queries have, and how deep the array types they declare nest,
/// and stops the visit with an error message beyond the limits
///
/// Within the limits, inferring the deepest definition found, 22 nested subqueries around
/// 64 `UNION`s and 40 comparisons of a cast to an array type 16 levels deep, took at
/// most 1552 KiB of stack in an unoptimized and 236 KiB in an optimized build (rustc
/// 1.100.0-nightly, x86_64 Linux).
#[derive(Default)]
struct Depth {
    expressions: usize,
    query_sources: usize,
    set_operations: usize,
}

impl Visitor for Depth {
    type Break = String;

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<String> {
        self.set_operations += set_operations(&query.body);
        if self.set_operations > MAX_SET_OPERATIONS {
            return ControlFlow::Break(format!(
                "Definitions of more than {MAX_SET_OPERATIONS} set operations"
            ));
        }
        query
            .with
            .iter()
            .flat_map(|with| &with.cte_tables)
            .try_for_each(|cte| check_alias(&cte.alias))
    }

    fn pre_visit_table_factor(&mut self, factor: &TableFactor) -> ControlFlow<String> {
        self.query_sources += 1;
        if self.query_sources > MAX_QUERY_SOURCE_DEPTH {
            return query_sources_too_deep();
        }
        check_query_source_types(factor, self.query_sources)
    }

    fn post_visit_table_factor(&mut self, _factor: &TableFactor) -> ControlFlow<String> {
        self.query_sources -= 1;
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<String> {
        self.expressions += 1;
        if self.expressions > MAX_EXPRESSION_DEPTH {
            return ControlFlow::Break(format!(
                "Expressions nested more than {MAX_EXPRESSION_DEPTH} levels deep"
            ));
        }
        match expr {
            Expr::Cast { data_type, .. }
            | Expr::Convert {
                data_type: Some(data_type),
                ..
            }
            | Expr::TypedString(TypedString { data_type, .. }) => check_type(data_type),
            Expr::Function(Function {
                args: FunctionArguments::List(list),
                ..
            }) => list.clauses.iter().try_for_each(|clause| match clause {
                FunctionArgumentClause::JsonReturningClause(returning) => {
                    check_type(&returning.data_type)
                }
                _ => ControlFlow::Continue(()),
            }),
            _ => ControlFlow::Continue(()),
        }
    }

    fn post_visit_expr(&mut self, _expr: &Expr) -> ControlFlow<String> {
        self.expressions -= 1;
        ControlFlow::Continue(())
    }
}

/// The number of set operations of `body`, without those of the queries in it
fn set_operations(mut body: &SetExpr) -> usize {
    let mut count = 0;
    // walk the left operands in a loop, as a chain nests them as deep as it is long
    while let SetExpr::SetOperation { left, right, .. } = body {
        count += 1 + set_operations(right);
        body = left;
    }
    count
}

/// Stops at a data type nesting more array types, like `INT[]`, `ARRAY<INT>`, or
/// `INT ARRAY`, than [`MAX_ARRAY_DEPTH`], or at a type like `TABLE(a INT)`
///
/// sqlparser reads such a `TABLE` type in every dialect, but no backend casts to one, and
/// the default of one of its columns can hold a cast nesting array types all over again.
fn check_type(mut data_type: &DataType) -> ControlFlow<String> {
    let mut depth = 0;
    while let DataType::Array(
        ArrayElemTypeDef::AngleBracket(element)
        | ArrayElemTypeDef::SquareBracket(element, _)
        | ArrayElemTypeDef::Qualified(element, _),
    ) = data_type
    {
        depth += 1;
        if depth > MAX_ARRAY_DEPTH {
            return ControlFlow::Break(format!(
                "Array types nested more than {MAX_ARRAY_DEPTH} levels deep"
            ));
        }
        data_type = element;
    }
    if let DataType::Table(_) = data_type {
        return ControlFlow::Break("Unsupported data type `TABLE`".to_owned());
    }
    ControlFlow::Continue(())
}

/// Checks the types an alias like `t(a INT[])` declares for its columns
fn check_alias(alias: &TableAlias) -> ControlFlow<String> {
    alias
        .columns
        .iter()
        .filter_map(|column| column.data_type.as_ref())
        .try_for_each(check_type)
}

/// Checks the types `factor` declares for its columns, in its alias or column list, where
/// `factor` nests `depth` query sources deep
fn check_query_source_types(factor: &TableFactor, depth: usize) -> ControlFlow<String> {
    let alias = match factor {
        TableFactor::Table { alias, .. }
        | TableFactor::Derived { alias, .. }
        | TableFactor::TableFunction { alias, .. }
        | TableFactor::Function { alias, .. }
        | TableFactor::UNNEST { alias, .. }
        | TableFactor::JsonTable { alias, .. }
        | TableFactor::OpenJsonTable { alias, .. }
        | TableFactor::NestedJoin { alias, .. }
        | TableFactor::Pivot { alias, .. }
        | TableFactor::Unpivot { alias, .. }
        | TableFactor::MatchRecognize { alias, .. }
        | TableFactor::XmlTable { alias, .. }
        | TableFactor::SemanticView { alias, .. } => alias.as_ref(),
        TableFactor::UnpivotExpr { .. } => None,
    };
    if let Some(alias) = alias {
        check_alias(alias)?;
    }
    match factor {
        TableFactor::XmlTable { columns, .. } => {
            columns.iter().try_for_each(|column| match &column.option {
                XmlTableColumnOption::NamedInfo { r#type, .. } => check_type(r#type),
                XmlTableColumnOption::ForOrdinality => ControlFlow::Continue(()),
            })
        }
        TableFactor::JsonTable { columns, .. } => check_json_table_columns(columns, depth),
        TableFactor::OpenJsonTable { columns, .. } => columns
            .iter()
            .try_for_each(|column| check_type(&column.r#type)),
        _ => ControlFlow::Continue(()),
    }
}

/// Checks the column types of a `JSON_TABLE` nested `depth` query sources deep, including
/// those of its nested columns, each of which nests another query source
fn check_json_table_columns(columns: &[JsonTableColumn], depth: usize) -> ControlFlow<String> {
    columns.iter().try_for_each(|column| match column {
        JsonTableColumn::Named(column) => check_type(&column.r#type),
        JsonTableColumn::ForOrdinality(_) => ControlFlow::Continue(()),
        JsonTableColumn::Nested(_) if depth >= MAX_QUERY_SOURCE_DEPTH => query_sources_too_deep(),
        JsonTableColumn::Nested(nested) => check_json_table_columns(&nested.columns, depth + 1),
    })
}

/// Stops the visit at query sources nested deeper than [`MAX_QUERY_SOURCE_DEPTH`]
fn query_sources_too_deep() -> ControlFlow<String> {
    ControlFlow::Break(format!(
        "Query sources nested more than {MAX_QUERY_SOURCE_DEPTH} levels deep"
    ))
}
