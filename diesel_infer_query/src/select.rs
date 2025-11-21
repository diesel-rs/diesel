// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::SchemaResolver;
use crate::error::Error;
use crate::error::Result;
use crate::query_source::QuerySource;
use sqlparser::ast::{SelectItem, SelectItemQualifiedWildcardKind};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub(crate) struct SelectField {
    pub(crate) ident: Option<String>,
    pub(crate) kind: Expression,
}

impl SelectField {
    pub(crate) fn infer_nullability(
        &self,
        resolver: &mut (dyn SchemaResolver + '_),
    ) -> Result<Option<bool>> {
        self.kind.infer_nullability(resolver)
    }
}

#[derive(Debug, PartialEq)]
/// WHEN [condition] THEN [result]` exrpession
pub(crate) struct CaseCondition {
    pub(crate) condition: Expression,
    pub(crate) result: Expression,
}

/// Different kind of expressions in a SELECT clause
#[derive(Debug, PartialEq)]
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
        query_source: String,
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
        relation: String,
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
    /// A unknown select expression
    Unknown,
}

impl Expression {
    pub(crate) fn infer_nullability(
        &self,
        resolver: &mut (dyn SchemaResolver + '_),
    ) -> Result<Option<bool>> {
        match self {
            Self::Literal { is_null, .. } => Ok(Some(*is_null)),
            Self::Field {
                via_left_join: true,
                ..
            } => Ok(Some(true)),
            Self::Field {
                schema,
                query_source: table,
                field_name,
                ..
            } => Ok(Some(
                resolver
                    .resolve_field(schema.as_deref(), table, field_name)
                    .map_err(|inner| Error::ResolverFailure { inner })?
                    .is_nullable(),
            )),
            Self::Cast { inner, .. } => inner.infer_nullability(resolver),
            Self::BinaryOp {
                left,
                right,
                statically_not_null,
                ..
            } => {
                if *statically_not_null {
                    Ok(Some(false))
                } else {
                    match (
                        left.infer_nullability(resolver)?,
                        right.infer_nullability(resolver)?,
                    ) {
                        (None, _) | (_, None) => Ok(None),
                        (Some(a), Some(b)) => Ok(Some(a || b)),
                    }
                }
            }
            Expression::PostfixOp {
                expr,
                statically_not_null,
                ..
            } => {
                if *statically_not_null {
                    Ok(Some(false))
                } else {
                    expr.infer_nullability(resolver)
                }
            }
            Expression::Function { name, schema, .. } => {
                match name.to_lowercase().as_str() {
                    // we consider count as only not nullable function for now
                    "count" if schema.is_none() => Ok(Some(false)),
                    _ => Ok(Some(true)),
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
                Ok(nullability
                    .into_iter()
                    .try_fold(false, |agg, v| Some(agg || v?)))
            }
            Expression::Unknown => Ok(None),
            Expression::Case {
                conditions,
                else_clause,
                ..
            } => conditions
                .iter()
                .map(|c| &c.result)
                .chain(else_clause.iter().map(|c| &**c))
                .map(|c| c.infer_nullability(resolver))
                .try_fold(Some(false), |agg, v| match (agg, v?) {
                    (Some(agg), Some(v)) => Ok(Some(agg || v)),
                    _ => Ok(None),
                }),
            Expression::Wildcard { .. } => unreachable!(),
        }
    }
}

pub(crate) fn infer_from_select(select: &sqlparser::ast::Select) -> Result<Vec<SelectField>> {
    let query_source_lookup = collect_query_sources(&select.from)?;

    select
        .projection
        .iter()
        .map(|p| infer_projection(p, &query_source_lookup))
        .collect()
}

pub(crate) fn collect_query_sources<'a>(
    sources: &'a [sqlparser::ast::TableWithJoins],
) -> Result<HashMap<&'a str, QuerySource<'a>>> {
    let mut out = HashMap::with_capacity(sources.len());
    for s in sources {
        QuerySource::fill_from_table_with_joins(&mut out, s)?;
    }
    Ok(out)
}

pub(crate) fn infer_projection(
    item: &SelectItem,
    query_source_lookup: &HashMap<&str, QuerySource>,
) -> Result<SelectField> {
    match item {
        SelectItem::UnnamedExpr(expr) => Ok(SelectField {
            ident: None,
            kind: crate::expression::infer_expr(expr, query_source_lookup)?,
        }),
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
                .and_then(|k| query_source_lookup.get(k))
            {
                let is_left_joined = item.contains_left_join(query_source_lookup)?;
                Ok(SelectField {
                    ident: None,
                    kind: Expression::Wildcard {
                        schema: item.schema.map(|s| s.to_owned()),
                        relation: item.name.to_owned(),
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
                    relation: wildcard.name.to_owned(),
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
    }
}
