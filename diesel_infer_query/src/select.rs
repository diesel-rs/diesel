// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::SchemaResolver;
use crate::IsNull;
use crate::backend::Backend;
use crate::error::Error;
use crate::error::Result;
use crate::functions::{FunctionKind, function_kind};
use crate::query_source::{QuerySource, find_query_source};
use sqlparser::ast::{
    Expr, Function, GroupByExpr, GroupByWithModifier, ObjectName, Query, SelectItem,
    SelectItemQualifiedWildcardKind, TableFactor, TableWithJoins, Visit, Visitor,
    visit_expressions,
};
use std::collections::HashMap;
use std::ops::ControlFlow;

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct SelectField {
    pub(crate) ident: Option<String>,
    pub(crate) kind: Expression,
}

impl SelectField {
    pub(crate) fn infer_nullability(
        &self,
        resolver: &mut (dyn SchemaResolver + '_),
    ) -> Result<IsNull> {
        self.kind.infer_nullability(resolver)
    }
}

#[derive(Debug, PartialEq, Clone)]
/// WHEN [condition] THEN [result]` exrpession
pub(crate) struct CaseCondition {
    pub(crate) condition: Expression,
    pub(crate) result: Expression,
}

/// Different kind of expressions in a SELECT clause
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum Expression {
    /// A literal value like `1` or `'foo'`
    Literal {
        /// The actual value
        value: Option<String>,
        /// is this value `NULL`?
        is_null: bool,
    },
    /// A field of a query source
    Field {
        /// the schema of the query source
        schema: Option<String>,
        /// the name of the query source
        query_source: Option<String>,
        /// the name of the field
        field_name: String,
        /// can the row this field is read from be missing, which makes the field
        /// `NULL` whatever its column says? That is the case for query sources joined
        /// via a `LEFT JOIN`, and for columns outside the aggregates of an aggregate
        /// query without `GROUP BY`
        nullable_row: bool,
    },
    /// A cast expression
    Cast {
        /// inner expression
        inner: Box<Expression>,
        /// target cast type
        tpe: String,
    },
    /// A binary operation
    BinaryOp {
        /// left side of the operation
        left: Box<Expression>,
        /// right side of the operation
        right: Box<Expression>,
        /// operator
        op: String,
        /// how the operation can produce null values
        nullability: OperatorNullability,
    },
    // A postfix operation like `IS NULL`
    PostfixOp {
        expr: Box<Expression>,
        op: String,
        nullability: OperatorNullability,
    },
    /// A function call
    Function {
        name: String,
        schema: Option<String>,
        arguments: Vec<Expression>,
    },
    /// A wild card `*` expression
    Wildcard {
        schema: Option<String>,
        relation: Option<String>,
        /// like `Field::nullable_row`
        nullable_row: bool,
    },
    /// A `left BETWEEN low AND high` expression
    Between {
        left: Box<Expression>,
        negated: bool,
        low: Box<Expression>,
        high: Box<Expression>,
    },
    // A `CASE [operand] [conditions] ELSE [else]` expression
    Case {
        operand: Option<Box<Expression>>,
        conditions: Vec<CaseCondition>,
        else_clause: Option<Box<Expression>>,
    },
    /// A `[left] (NOT) IN ([LIST]) expression
    In {
        left: Box<Expression>,
        negated: bool,
        list: Vec<Expression>,
    },
    /// A `[left] (NOT) IN ([QUERY]) expression
    InSubQuery {
        left: Box<Expression>,
        negated: bool,
        subquery: Vec<SelectField>,
    },
    /// An expression grouping by `(inner)`
    Grouped(Box<Expression>),
    /// SubQuery,
    Subquery { selection: Vec<SelectField> },
    /// A field based on a combined set
    Combined {
        left: Box<Expression>,
        right: Box<Expression>,
        set_operator: SetOperator,
    },
    /// A unknown select expression
    Unknown,
}

/// How the result of an operator can be `NULL`
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OperatorNullability {
    /// Never `NULL`, like `IS NULL`
    NeverNull,
    /// `NULL` exactly when an operand is `NULL`, like `=` or `||`
    NullIfOperandNull,
    /// Can be `NULL` for any operands, like `/` for a zero divisor
    Nullable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SetOperator {
    Union,
    Intersect,
    Except,
}

impl SetOperator {
    fn from_sql_parser_ast(ast: &sqlparser::ast::SetOperator) -> Result<Self> {
        match ast {
            sqlparser::ast::SetOperator::Union => Ok(Self::Union),
            sqlparser::ast::SetOperator::Except => Ok(Self::Except),
            sqlparser::ast::SetOperator::Intersect => Ok(Self::Intersect),
            sqlparser::ast::SetOperator::Minus => Err(Error::UnsupportedSql {
                msg: format!("Unsupported set operator: {ast}"),
            }),
        }
    }
}

impl Expression {
    pub(crate) fn infer_nullability(
        &self,
        resolver: &mut (dyn SchemaResolver + '_),
    ) -> Result<IsNull> {
        match self {
            Self::Literal { is_null, .. } => Ok(if *is_null {
                IsNull::IsNullable
            } else {
                IsNull::NotNullable
            }),
            Self::Field {
                nullable_row: true, ..
            } => Ok(IsNull::IsNullable),
            Self::Field {
                schema,
                query_source: table,
                field_name,
                ..
            } => Ok(resolver
                .resolve_field(schema.as_deref(), table.as_deref(), field_name)
                .map_err(|inner| Error::ResolverFailure { inner })?
                .is_nullable()),
            Self::Cast { inner, .. } => inner.infer_nullability(resolver),
            Self::BinaryOp {
                left,
                right,
                nullability,
                ..
            } => match nullability {
                OperatorNullability::NeverNull => Ok(IsNull::NotNullable),
                OperatorNullability::Nullable => Ok(IsNull::IsNullable),
                OperatorNullability::NullIfOperandNull => Ok(left
                    .infer_nullability(resolver)?
                    .or(right.infer_nullability(resolver)?)),
            },
            Expression::PostfixOp {
                expr, nullability, ..
            } => match nullability {
                OperatorNullability::NeverNull => Ok(IsNull::NotNullable),
                OperatorNullability::Nullable => Ok(IsNull::IsNullable),
                OperatorNullability::NullIfOperandNull => expr.infer_nullability(resolver),
            },
            Expression::Function {
                name,
                schema,
                arguments,
            } => {
                match name.to_lowercase().as_str() {
                    // we consider count as only not nullable function for now
                    "count" if schema.is_none() => Ok(IsNull::NotNullable),
                    // coalesce is not nullable if any argument cannot contain
                    // null values as it returns the first non-null value
                    "coalesce" if schema.is_none() => {
                        for arg in arguments {
                            match arg.infer_nullability(resolver)? {
                                IsNull::NotNullable => return Ok(IsNull::NotNullable),
                                IsNull::IsNullable | IsNull::Unknown => {}
                            }
                        }
                        Ok(IsNull::IsNullable)
                    }
                    _ => Ok(IsNull::IsNullable),
                }
            }
            Expression::Between {
                left, low, high, ..
            } => {
                let nullability = [
                    left.infer_nullability(resolver)?,
                    low.infer_nullability(resolver)?,
                    high.infer_nullability(resolver)?,
                ];
                Ok(nullability[0].or(nullability[1]).or(nullability[2]))
            }
            // Without an `ELSE` the result is `NULL` when no branch matches
            Expression::Case {
                else_clause: None, ..
            } => Ok(IsNull::IsNullable),
            Expression::Case {
                conditions,
                else_clause: Some(else_clause),
                ..
            } => conditions
                .iter()
                .map(|c| &c.result)
                .chain([&**else_clause])
                .map(|c| c.infer_nullability(resolver))
                .try_fold(IsNull::NotNullable, |agg, v| Ok(agg.or(v?))),
            Expression::In { left, list, .. } => {
                let left = left.infer_nullability(resolver);
                list.iter()
                    .map(|l| l.infer_nullability(resolver))
                    .chain([left])
                    .try_fold(IsNull::NotNullable, |agg, v| Ok(agg.or(v?)))
            }
            Expression::Grouped(inner) => inner.infer_nullability(resolver),
            Expression::Subquery { selection } => match selection.as_slice() {
                [_s] => {
                    // always nullable as the subquery might return an empty result
                    Ok(IsNull::IsNullable)
                }
                _ => Ok(IsNull::Unknown),
            },
            Expression::InSubQuery { left, subquery, .. } => {
                match subquery.as_slice() {
                    [s] => {
                        // `x IN (SELECT y …)` yields NULL when x matches no row and the
                        // subquery contains a NULL, so the result is nullable whenever
                        // either side is nullable
                        Ok(left
                            .infer_nullability(resolver)?
                            .or(s.infer_nullability(resolver)?))
                    }
                    // A subquery should return only one column
                    // if that's not the case, give up
                    _ => Ok(IsNull::Unknown),
                }
            }
            // for except set operations only the left
            // side is actually returned
            Expression::Combined {
                left,
                set_operator: SetOperator::Except,
                ..
            } => left.infer_nullability(resolver),
            // for `UNION` set operations both sides are
            // returned, so if any side can contain
            // null values we need to assume this for the
            // whole expression
            Expression::Combined {
                left,
                right,
                set_operator: SetOperator::Union,
            } => {
                let left = left.infer_nullability(resolver)?;
                let right = right.infer_nullability(resolver)?;
                Ok(left.or(right))
            }
            // for `INTERSECT` we only return values
            // that are in both result sets, so
            // if any side is not nullable we only return
            // not nullable values for this expression
            Expression::Combined {
                left,
                right,
                set_operator: SetOperator::Intersect,
            } => {
                let left = left.infer_nullability(resolver)?;
                let right = right.infer_nullability(resolver)?;
                let ret = match (left, right) {
                    (IsNull::Unknown, _) | (_, IsNull::Unknown) => IsNull::Unknown,
                    (IsNull::NotNullable, _) | (_, IsNull::NotNullable) => IsNull::NotNullable,
                    (IsNull::IsNullable, IsNull::IsNullable) => IsNull::IsNullable,
                };
                Ok(ret)
            }
            Expression::Wildcard { .. } => Err(Error::UnresolvedWildcard),
            Expression::Unknown => Ok(IsNull::Unknown),
        }
    }

    /// Mark every field this expression reads as coming from a row that can be missing
    fn mark_rows_nullable(&mut self) {
        match self {
            Self::Field { nullable_row, .. } | Self::Wildcard { nullable_row, .. } => {
                *nullable_row = true
            }
            Self::Cast { inner, .. } | Self::Grouped(inner) => inner.mark_rows_nullable(),
            Self::PostfixOp { expr, .. } => expr.mark_rows_nullable(),
            Self::BinaryOp { left, right, .. } | Self::Combined { left, right, .. } => {
                left.mark_rows_nullable();
                right.mark_rows_nullable();
            }
            Self::Between {
                left, low, high, ..
            } => {
                left.mark_rows_nullable();
                low.mark_rows_nullable();
                high.mark_rows_nullable();
            }
            Self::Case {
                operand,
                conditions,
                else_clause,
            } => {
                for e in operand.iter_mut().chain(else_clause.iter_mut()) {
                    e.mark_rows_nullable();
                }
                for c in conditions {
                    c.condition.mark_rows_nullable();
                    c.result.mark_rows_nullable();
                }
            }
            // `coalesce()` depends on the nullability of its arguments
            Self::Function { arguments, .. } => {
                arguments.iter_mut().for_each(Self::mark_rows_nullable)
            }
            Self::In { left, list, .. } => {
                left.mark_rows_nullable();
                list.iter_mut().for_each(Self::mark_rows_nullable);
            }
            // a subquery can read the missing row too
            Self::InSubQuery { left, subquery, .. } => {
                left.mark_rows_nullable();
                for field in subquery {
                    field.kind.mark_rows_nullable();
                }
            }
            Self::Subquery { selection } => {
                for field in selection {
                    field.kind.mark_rows_nullable();
                }
            }
            Self::Literal { .. } | Self::Unknown => {}
        }
    }
}

pub(crate) fn infer_from_select(
    select: &sqlparser::ast::Select,
    outer_lookup: Option<&HashMap<Option<&str>, QuerySource>>,
    backend: Backend,
) -> Result<Vec<SelectField>> {
    let mut query_source_lookup = collect_query_sources(&select.from)?;
    if let Some(outer) = outer_lookup {
        for (k, v) in outer {
            if !query_source_lookup.contains_key(k) {
                query_source_lookup.insert(*k, v.clone());
            }
        }
    }

    let mut fields = select
        .projection
        .iter()
        .map(|p| infer_projection(p, &query_source_lookup, backend))
        .collect::<Result<Vec<_>>>()?;
    if bare_columns_can_be_null(select, backend) {
        for field in &mut fields {
            field.kind.mark_rows_nullable();
        }
    }
    Ok(fields)
}

/// Can a row of `select` have `NULL` for a column read outside the aggregates?
///
/// An aggregate query without `GROUP BY` returns one row even for empty input, and
/// grouping sets add super-aggregate rows with `NULL` for the grouping columns.
fn bare_columns_can_be_null(select: &sqlparser::ast::Select, backend: Backend) -> bool {
    if has_grouping_sets(&select.group_by) {
        return true;
    }

    let grouped =
        !matches!(&select.group_by, GroupByExpr::Expressions(exprs, _) if exprs.is_empty());
    !grouped && (has_aggregate_for_outer_select(select, backend) || select.having.is_some())
}

fn has_grouping_sets(group_by: &GroupByExpr) -> bool {
    match group_by {
        GroupByExpr::All(modifiers) => modifiers.iter().any(is_grouping_set_modifier),
        GroupByExpr::Expressions(expressions, modifiers) => {
            modifiers.iter().any(is_grouping_set_modifier)
                || visit_expressions(expressions, |expr| match expr {
                    Expr::Rollup(_) | Expr::Cube(_) | Expr::GroupingSets(_) => {
                        ControlFlow::Break(())
                    }
                    Expr::Function(function)
                        if object_name_is(&function.name, "rollup")
                            || object_name_is(&function.name, "cube") =>
                    {
                        ControlFlow::Break(())
                    }
                    _ => ControlFlow::Continue(()),
                })
                .is_break()
        }
    }
}

fn is_grouping_set_modifier(modifier: &GroupByWithModifier) -> bool {
    matches!(
        modifier,
        GroupByWithModifier::Rollup
            | GroupByWithModifier::Cube
            | GroupByWithModifier::GroupingSets(_)
    )
}

fn has_aggregate_for_outer_select(select: &sqlparser::ast::Select, backend: Backend) -> bool {
    let mut outer_names = Vec::new();
    for source in &select.from {
        collect_exposed_names(source, &mut outer_names);
    }
    let mut visitor = AggregateVisitor {
        backend,
        outer_names,
        nested_query_sources: Vec::new(),
    };
    if select.projection.visit(&mut visitor).is_break() {
        return true;
    }
    select.named_window.visit(&mut visitor).is_break()
}

struct AggregateVisitor {
    backend: Backend,
    /// names the FROM clause of the outer SELECT exposes
    outer_names: Vec<String>,
    nested_query_sources: Vec<Vec<String>>,
}

impl AggregateVisitor {
    fn can_aggregate(&self, function: &Function) -> bool {
        match function_kind(function, self.backend) {
            FunctionKind::Aggregate => true,
            FunctionKind::Scalar => false,
            // PostgreSQL rejects columns outside aggregates in a query without `GROUP BY`
            // that aggregates, so in its views every function next to such a column is
            // scalar
            FunctionKind::Unknown => self.backend != Backend::Pg,
        }
    }
}

impl Visitor for AggregateVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        self.nested_query_sources
            .push(exposed_names_in_query(query));
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &Query) -> ControlFlow<Self::Break> {
        let _ = self.nested_query_sources.pop();
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        if let Expr::Function(function) = expr
            && function.over.is_none()
            && self.can_aggregate(function)
            && (self.nested_query_sources.is_empty()
                || !aggregate_is_owned_by_nested_query(
                    function,
                    &self.outer_names,
                    &self.nested_query_sources,
                ))
        {
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }
}

fn exposed_names_in_query(query: &Query) -> Vec<String> {
    let Some(select) = query.body.as_select() else {
        return Vec::new();
    };

    let mut names = Vec::new();
    for source in &select.from {
        collect_exposed_names(source, &mut names);
    }
    names
}

fn collect_exposed_names(source: &TableWithJoins, names: &mut Vec<String>) {
    collect_exposed_name(&source.relation, names);
    for join in &source.joins {
        collect_exposed_name(&join.relation, names);
    }
}

fn collect_exposed_name(table: &TableFactor, names: &mut Vec<String>) {
    match table {
        TableFactor::Table { name, alias, .. } => {
            if let Some(alias) = alias {
                names.push(alias.name.value.clone());
            } else if let Some(name) = object_name_last_ident(name) {
                names.push(name.to_owned());
            }
        }
        TableFactor::Derived {
            alias: Some(alias), ..
        } => names.push(alias.name.value.clone()),
        TableFactor::NestedJoin {
            table_with_joins,
            alias,
        } => {
            if let Some(alias) = alias {
                names.push(alias.name.value.clone());
            } else {
                collect_exposed_names(table_with_joins, names);
            }
        }
        _ => {}
    }
}

fn aggregate_is_owned_by_nested_query(
    function: &Function,
    outer_names: &[String],
    nested_query_sources: &[Vec<String>],
) -> bool {
    let mut visitor = ColumnReferenceVisitor {
        outer_names,
        nested_query_sources,
        argument_queries: 0,
        has_reference: false,
        has_nested_reference: false,
    };
    let _ = function.args.visit(&mut visitor);
    let _ = function.filter.visit(&mut visitor);
    let _ = function.within_group.visit(&mut visitor);
    !visitor.has_reference || visitor.has_nested_reference
}

struct ColumnReferenceVisitor<'a> {
    outer_names: &'a [String],
    nested_query_sources: &'a [Vec<String>],
    /// how many queries inside the aggregate's arguments enclose the visited expression
    argument_queries: usize,
    has_reference: bool,
    has_nested_reference: bool,
}

impl ColumnReferenceVisitor<'_> {
    fn record_qualified_name(&mut self, name: &str) {
        self.has_reference = true;
        // inside a query of the arguments the name can refer to that query, which says
        // nothing about the level the aggregate belongs to; and when a subquery's
        // relation of that name lacks the column, SQLite looks the column up in the outer
        // SELECT, so a name the outer SELECT exposes too, in any ASCII case, proves nothing
        if self.argument_queries == 0
            && !self
                .outer_names
                .iter()
                .any(|outer| outer.eq_ignore_ascii_case(name))
        {
            self.has_nested_reference |= self
                .nested_query_sources
                .iter()
                .any(|query| query.iter().any(|source| source == name));
        }
    }
}

impl Visitor for ColumnReferenceVisitor<'_> {
    type Break = ();

    fn pre_visit_query(&mut self, _query: &Query) -> ControlFlow<Self::Break> {
        self.argument_queries += 1;
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &Query) -> ControlFlow<Self::Break> {
        self.argument_queries -= 1;
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        match expr {
            Expr::Identifier(_) => self.has_reference = true,
            Expr::CompoundIdentifier(identifiers) => {
                self.has_reference = true;
                if let Some(qualifier) = identifiers.iter().rev().nth(1) {
                    self.record_qualified_name(&qualifier.value);
                }
            }
            // only PostgreSQL accepts qualified wildcards like `t.*` in expressions, and it
            // rejects bare columns next to an aggregate of the outer SELECT, so where such
            // an aggregate belongs cannot change what it makes nullable
            _ => {}
        }
        ControlFlow::Continue(())
    }
}

fn object_name_is(name: &ObjectName, expected: &str) -> bool {
    object_name_last_ident(name).is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn object_name_last_ident(name: &ObjectName) -> Option<&str> {
    name.0
        .last()
        .and_then(|part| part.as_ident())
        .map(|ident| ident.value.as_str())
}

pub(crate) fn collect_query_sources<'a>(
    sources: &'a [sqlparser::ast::TableWithJoins],
) -> Result<HashMap<Option<&'a str>, QuerySource<'a>>> {
    let mut out = HashMap::with_capacity(sources.len());
    for s in sources {
        QuerySource::fill_from_table_with_joins(&mut out, s)?;
    }
    Ok(out)
}

pub(crate) fn infer_projection(
    item: &SelectItem,
    query_source_lookup: &HashMap<Option<&str>, QuerySource>,
    backend: Backend,
) -> Result<SelectField> {
    match item {
        SelectItem::UnnamedExpr(expr) => {
            let kind = crate::expression::infer_expr(expr, query_source_lookup, backend)?;
            let ident = if let Expression::Field { field_name, .. } = &kind {
                Some(field_name.clone())
            } else {
                None
            };
            Ok(SelectField { ident, kind })
        }
        SelectItem::ExprWithAlias { expr, alias } => Ok(SelectField {
            ident: Some(alias.value.clone()),
            kind: crate::expression::infer_expr(expr, query_source_lookup, backend)?,
        }),
        SelectItem::QualifiedWildcard(
            SelectItemQualifiedWildcardKind::ObjectName(name),
            _wildcard_additional_options,
        ) => {
            if let Some(item) = name
                .0
                .last()
                .and_then(|a| a.as_ident())
                .map(|a| a.value.as_str())
                .and_then(|k| find_query_source(query_source_lookup, k))
            {
                let nullable_row = item.contains_left_join(query_source_lookup)?;
                Ok(SelectField {
                    ident: None,
                    kind: Expression::Wildcard {
                        schema: item.schema.map(|s| s.to_owned()),
                        relation: item.name.map(|c| c.to_owned()),
                        nullable_row,
                    },
                })
            } else {
                Err(Error::InvalidQuerySource {
                    query_source: name.to_string(),
                })
            }
        }
        SelectItem::Wildcard(_) if query_source_lookup.len() == 1 => {
            let wildcard = query_source_lookup.values().next().expect("Is exactly one");
            Ok(SelectField {
                ident: None,
                kind: Expression::Wildcard {
                    schema: wildcard.schema.map(|c| c.to_owned()),
                    relation: wildcard.name.map(|c| c.to_owned()),
                    nullable_row: false,
                },
            })
        }
        s @ SelectItem::Wildcard(_wildcard_additional_options) => Err(Error::UnsupportedSql {
            msg: format!("Unsupported `SELECT` expression: `{s}`"),
        }),
        s @ SelectItem::QualifiedWildcard(
            _select_item_qualified_wildcard_kind,
            _wildcard_additional_options,
        ) => Err(Error::UnsupportedSql {
            msg: format!("Unsupported `SELECT` expression: `{s}`"),
        }),
        s @ SelectItem::ExprWithAliases { .. } => Err(Error::UnsupportedSql {
            msg: format!("Unsupported `SELECT` expression: `{s}`"),
        }),
    }
}

pub(crate) fn parse_query(
    select: &sqlparser::ast::Query,
    outer_lookup: Option<&HashMap<Option<&str>, QuerySource>>,
    backend: Backend,
) -> Result<Vec<SelectField>> {
    let expr = &select.body;
    parse_from_set_expr(outer_lookup, expr, backend)
}

fn parse_from_set_expr(
    outer_lookup: Option<&HashMap<Option<&str>, QuerySource<'_>>>,
    expr: &sqlparser::ast::SetExpr,
    backend: Backend,
) -> Result<Vec<SelectField>> {
    match expr {
        sqlparser::ast::SetExpr::Select(select_expr) => {
            infer_from_select(select_expr, outer_lookup, backend)
        }
        // Set expressions like `UNION`, `INTERSECT` and `EXCEPT`
        sqlparser::ast::SetExpr::SetOperation {
            left, op, right, ..
        } => {
            let op = SetOperator::from_sql_parser_ast(op)?;
            let left = parse_from_set_expr(outer_lookup, left, backend)?;
            let right = parse_from_set_expr(outer_lookup, right, backend)?;
            if left.len() == right.len() {
                Ok(left
                    .into_iter()
                    .zip(right)
                    .map(|(l, r)| SelectField {
                        ident: l.ident,
                        kind: Expression::Combined {
                            left: Box::new(l.kind),
                            right: Box::new(r.kind),
                            set_operator: op,
                        },
                    })
                    .collect())
            } else {
                Err(Error::UnsupportedSql {
                    msg: format!(
                        "We expect set operations to have the same number of fields on both sides. \
                         We got {} fields on the left side and {} fields on the right side",
                        left.len(),
                        right.len()
                    ),
                })
            }
        }
        s => Err(Error::UnsupportedSql {
            msg: format!("Unsupported query kind: `{s}`"),
        }),
    }
}
