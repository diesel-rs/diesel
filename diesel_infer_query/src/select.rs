// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::IsNull;
use crate::error::Error;
use crate::error::Result;
use crate::query_source::QuerySource;
use crate::resolver::{CombinedResolver, SchemaResolver};
use crate::views::ParseContext;
use sqlparser::ast::{SelectItem, SelectItemQualifiedWildcardKind};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub(crate) struct SelectField {
    pub(crate) ident: Option<String>,
    pub(crate) kind: Expression,
}

impl SelectField {
    pub(crate) fn infer_nullability(&self, resolver: &mut CombinedResolver<'_>) -> Result<IsNull> {
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
        query_source: Option<String>,
        /// the name of the field
        source_id: Option<usize>,
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
        source_id: Option<usize>,
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
    pub(crate) fn infer_nullability(&self, resolver: &mut CombinedResolver<'_>) -> Result<IsNull> {
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
                source_id,
                field_name,
                ..
            } => {
                let field = match source_id {
                    Some(id) => {
                        resolver.resolve_derived_field(*id, table.as_deref(), field_name)?
                    }
                    None => resolver
                        .resolve_field(schema.as_deref(), table.as_deref(), field_name)
                        .map_err(|inner| Error::ResolverFailure { inner })?,
                };
                Ok(field.is_nullable())
            }
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
    context: &mut ParseContext,
) -> Result<Vec<SelectField>> {
    let mut query_source_lookup = collect_query_sources(&select.from)?;
    for source in &select.from {
        register_derived(&source.relation, &mut query_source_lookup, context)?;
        for join in &source.joins {
            register_derived(&join.relation, &mut query_source_lookup, context)?;
        }
    }
    for source in query_source_lookup.values_mut() {
        if source.source_id.is_none() && source.schema.is_none() {
            source.source_id = source.name.and_then(|name| context.cte(name));
        }
    }
    if let Some(outer) = outer_lookup {
        for (k, v) in outer {
            query_source_lookup.entry(*k).or_insert_with(|| v.clone());
        }
    }
    select
        .projection
        .iter()
        .map(|p| infer_projection(p, &query_source_lookup, context))
        .collect()
}

fn register_derived<'a>(
    source: &'a sqlparser::ast::TableFactor,
    lookup: &mut HashMap<Option<&'a str>, QuerySource<'a>>,
    context: &mut ParseContext,
) -> Result<()> {
    match source {
        sqlparser::ast::TableFactor::Derived {
            subquery, alias, ..
        } => {
            let fields = parse_query(subquery, None, context)?;
            let id = context.register(fields);
            let key = alias.as_ref().map(|alias| alias.name.value.as_str());
            if let Some(source) = lookup.get_mut(&key) {
                source.source_id = Some(id);
            }
        }
        sqlparser::ast::TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            register_derived(&table_with_joins.relation, lookup, context)?;
            for join in &table_with_joins.joins {
                register_derived(&join.relation, lookup, context)?;
            }
        }
        _ => {}
    }
    Ok(())
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
    context: &mut ParseContext,
) -> Result<SelectField> {
    match item {
        SelectItem::UnnamedExpr(expr) => {
            let kind = crate::expression::infer_expr(expr, query_source_lookup, context)?;
            let ident = if let Expression::Field { field_name, .. } = &kind {
                Some(field_name.clone())
            } else {
                None
            };
            Ok(SelectField { ident, kind })
        }
        SelectItem::ExprWithAlias { expr, alias } => Ok(SelectField {
            ident: Some(alias.value.clone()),
            kind: crate::expression::infer_expr(expr, query_source_lookup, context)?,
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
                        source_id: item.source_id,
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
                    source_id: wildcard.source_id,
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
    context: &mut ParseContext,
) -> Result<Vec<SelectField>> {
    let outer_ctes = context.ctes.len();
    if let Some(with) = &select.with {
        for cte in &with.cte_tables {
            let mut fields = parse_query(&cte.query, None, context)?;
            if !cte.alias.columns.is_empty() {
                if fields.len() != cte.alias.columns.len() {
                    return Err(Error::UnsupportedSql {
                        msg: format!(
                            "Not matching field count for a CTE. Got {} fields, but expected {} fields",
                            fields.len(),
                            cte.alias.columns.len()
                        ),
                    });
                }
                for (field, alias) in fields.iter_mut().zip(&cte.alias.columns) {
                    field.ident = Some(alias.name.value.clone());
                }
            }
            let id = context.register(fields);
            context.ctes.push((cte.alias.name.value.clone(), id));
        }
    }
    let result = parse_from_set_expr(outer_lookup, &select.body, context);
    context.ctes.truncate(outer_ctes);
    result
}

fn parse_from_set_expr(
    outer_lookup: Option<&HashMap<Option<&str>, QuerySource<'_>>>,
    expr: &sqlparser::ast::SetExpr,
    context: &mut ParseContext,
) -> Result<Vec<SelectField>> {
    match expr {
        sqlparser::ast::SetExpr::Select(select_expr) => {
            infer_from_select(select_expr, outer_lookup, context)
        }
        // Set expressions like `UNION`, `INTERSECT` and `EXCEPT`
        sqlparser::ast::SetExpr::SetOperation {
            left, op, right, ..
        } => {
            let op = SetOperator::from_sql_parser_ast(op)?;
            let left = parse_from_set_expr(outer_lookup, left, context)?;
            let right = parse_from_set_expr(outer_lookup, right, context)?;
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
