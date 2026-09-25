// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::SchemaResolver;
use crate::IsNull;
use crate::error::Error;
use crate::error::Result;
use crate::query_source::QuerySource;
use sqlparser::ast::{SelectItem, SelectItemQualifiedWildcardKind};
use std::collections::HashMap;

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
        /// is this field coming from a query source joined via a `LEFT JOIN`
        via_left_join: bool,
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
        /// is this operation statically known not to produce null values
        statically_not_null: bool,
    },
    // A postfix operation like `IS NULL`
    PostfixOp {
        expr: Box<Expression>,
        op: String,
        statically_not_null: bool,
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
        is_left_joined: bool,
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
                via_left_join: true,
                ..
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
                statically_not_null,
                ..
            } => {
                if *statically_not_null {
                    Ok(IsNull::NotNullable)
                } else {
                    Ok(left
                        .infer_nullability(resolver)?
                        .or(right.infer_nullability(resolver)?))
                }
            }
            Expression::PostfixOp {
                expr,
                statically_not_null,
                ..
            } => {
                if *statically_not_null {
                    Ok(IsNull::NotNullable)
                } else {
                    expr.infer_nullability(resolver)
                }
            }
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
            Expression::Case {
                conditions,
                else_clause,
                ..
            } => conditions
                .iter()
                .map(|c| &c.result)
                .chain(else_clause.iter().map(|c| &**c))
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
}

pub(crate) fn infer_from_select(
    select: &sqlparser::ast::Select,
    outer_lookup: Option<&HashMap<Option<&str>, QuerySource>>,
) -> Result<Vec<SelectField>> {
    let mut query_source_lookup = collect_query_sources(&select.from)?;
    if let Some(outer) = outer_lookup {
        for (k, v) in outer {
            if !query_source_lookup.contains_key(k) {
                query_source_lookup.insert(*k, v.clone());
            }
        }
    }

    select
        .projection
        .iter()
        .map(|p| infer_projection(p, &query_source_lookup))
        .collect()
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
) -> Result<SelectField> {
    match item {
        SelectItem::UnnamedExpr(expr) => {
            let kind = crate::expression::infer_expr(expr, query_source_lookup)?;
            let ident = if let Expression::Field { field_name, .. } = &kind {
                Some(field_name.clone())
            } else {
                None
            };
            Ok(SelectField { ident, kind })
        }
        SelectItem::ExprWithAlias { expr, alias } => Ok(SelectField {
            ident: Some(alias.value.clone()),
            kind: crate::expression::infer_expr(expr, query_source_lookup)?,
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
                .and_then(|k| query_source_lookup.get(&Some(k)))
            {
                let is_left_joined = item.contains_left_join(query_source_lookup)?;
                Ok(SelectField {
                    ident: None,
                    kind: Expression::Wildcard {
                        schema: item.schema.map(|s| s.to_owned()),
                        relation: item.name.map(|c| c.to_owned()),
                        is_left_joined,
                    },
                })
            } else {
                todo!()
            }
        }
        SelectItem::Wildcard(_) if query_source_lookup.len() == 1 => {
            let wildcard = query_source_lookup.values().next().expect("Is exactly one");
            Ok(SelectField {
                ident: None,
                kind: Expression::Wildcard {
                    schema: wildcard.schema.map(|c| c.to_owned()),
                    relation: wildcard.name.map(|c| c.to_owned()),
                    is_left_joined: false,
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
) -> Result<Vec<SelectField>> {
    let expr = &select.body;
    parse_from_set_expr(outer_lookup, expr)
}

fn parse_from_set_expr(
    outer_lookup: Option<&HashMap<Option<&str>, QuerySource<'_>>>,
    expr: &sqlparser::ast::SetExpr,
) -> Result<Vec<SelectField>> {
    match expr {
        sqlparser::ast::SetExpr::Select(select_expr) => {
            infer_from_select(select_expr, outer_lookup)
        }
        // Set expressions like `UNION`, `INTERSECT` and `EXCEPT`
        sqlparser::ast::SetExpr::SetOperation {
            left, op, right, ..
        } => {
            let op = SetOperator::from_sql_parser_ast(op)?;
            let left = parse_from_set_expr(outer_lookup, left)?;
            let right = parse_from_set_expr(outer_lookup, right)?;
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
