// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::backend::Backend;
use crate::error::{Error, Result};
use crate::query_source::{QuerySource, find_query_source};
use crate::select::{CaseCondition, Expression, OperatorNullability};
use sqlparser::ast::{
    BinaryOperator, Expr, FunctionArg, FunctionArgExpr, FunctionArguments, ObjectNamePart,
    TableFactor, UnaryOperator, Value, Visit, Visitor,
};
use std::collections::HashMap;
use std::ops::ControlFlow;

pub(crate) fn infer_expr(
    expr: &sqlparser::ast::Expr,
    query_source_lookup: &HashMap<Option<&str>, QuerySource>,
    backend: Backend,
) -> Result<Expression> {
    match expr {
        Expr::Value(v) => Ok(Expression::Literal {
            value: v.value.clone().into_string(),
            is_null: matches!(v.value, Value::Null),
        }),
        Expr::Identifier(id) => {
            if query_source_lookup.len() == 1 {
                let table = query_source_lookup
                    .values()
                    .next()
                    .expect("We checked there is only one value");
                Ok(Expression::Field {
                    schema: table.schema.map(|s| s.to_owned()),
                    query_source: table.name.map(|s| s.to_owned()),
                    field_name: id.value.clone(),
                    // no joins here so we should be fine
                    nullable_row: false,
                })
            } else {
                Err(Error::UnsupportedSql {
                    msg: format!(
                        "Queries without more than one table are not supported yet: `{id}`"
                    ),
                })
            }
        }
        s @ Expr::CompoundIdentifier(ids) => match ids.as_slice() {
            [table, field] => {
                let table =
                    find_query_source(query_source_lookup, &table.value).ok_or_else(|| {
                        Error::InvalidQuerySource {
                            query_source: table.value.clone(),
                        }
                    })?;
                // lookup if this field comes in via a `LEFT JOIN` somewhere in the chain
                let nullable_row = table.contains_left_join(query_source_lookup)?;
                Ok(Expression::Field {
                    schema: table.schema.map(|s| s.to_owned()),
                    query_source: table.name.map(|s| s.to_owned()),
                    field_name: field.value.clone(),
                    nullable_row,
                })
            }
            _ => Err(Error::UnsupportedSql {
                msg: format!(
                    "Queries with more than 2 identifier segments are not supported yet: `{s}`"
                ),
            }),
        },
        Expr::Cast {
            expr, data_type, ..
        } => {
            let inner = infer_expr(expr, query_source_lookup, backend)?;
            Ok(Expression::Cast {
                inner: Box::new(inner),
                tpe: data_type.to_string(),
            })
        }
        Expr::BinaryOp { left, op, right } => {
            let left = Box::new(infer_expr(left, query_source_lookup, backend)?);
            let right = Box::new(infer_expr(right, query_source_lookup, backend)?);
            Ok(Expression::BinaryOp {
                left,
                right,
                op: op.to_string(),
                nullability: binary_operator_nullability(op),
            })
        }
        Expr::IsNull(e) => {
            let inner = Box::new(infer_expr(e, query_source_lookup, backend)?);
            Ok(Expression::PostfixOp {
                expr: inner,
                op: String::from("IS NULL"),
                nullability: OperatorNullability::NeverNull,
            })
        }
        Expr::IsNotNull(e) => {
            let inner = Box::new(infer_expr(e, query_source_lookup, backend)?);
            Ok(Expression::PostfixOp {
                expr: inner,
                op: String::from("IS NOT NULL"),
                nullability: OperatorNullability::NeverNull,
            })
        }
        Expr::Function(f) => infer_functions(f, query_source_lookup, backend),
        Expr::Like {
            negated,
            any,
            expr,
            pattern,
            escape_char,
        } if !*any && escape_char.is_none() => {
            let op = if *negated { "NOT LIKE" } else { "LIKE" };
            Ok(Expression::BinaryOp {
                left: Box::new(infer_expr(expr, query_source_lookup, backend)?),
                right: Box::new(infer_expr(pattern, query_source_lookup, backend)?),
                op: String::from(op),
                nullability: pattern_nullability(backend),
            })
        }
        Expr::ILike {
            negated,
            any,
            expr,
            pattern,
            escape_char,
        } if !*any && escape_char.is_none() => {
            let op = if *negated { "NOT ILIKE" } else { "ILIKE" };
            Ok(Expression::BinaryOp {
                left: Box::new(infer_expr(expr, query_source_lookup, backend)?),
                right: Box::new(infer_expr(pattern, query_source_lookup, backend)?),
                op: String::from(op),
                nullability: pattern_nullability(backend),
            })
        }
        // SQLite reads `a IS DISTINCT FROM b = c` as `(a IS DISTINCT FROM b) = c`, but
        // sqlparser takes the comparison after `FROM` as the right operand
        Expr::IsDistinctFrom(_, b) | Expr::IsNotDistinctFrom(_, b) if !binds_tighter_than_is(b) => {
            Ok(Expression::Unknown)
        }
        Expr::IsDistinctFrom(a, b) => Ok(Expression::BinaryOp {
            left: Box::new(infer_expr(a, query_source_lookup, backend)?),
            right: Box::new(infer_expr(b, query_source_lookup, backend)?),
            op: String::from("IS DISTINCT FROM"),
            nullability: OperatorNullability::NeverNull,
        }),
        Expr::IsNotDistinctFrom(a, b) => Ok(Expression::BinaryOp {
            left: Box::new(infer_expr(a, query_source_lookup, backend)?),
            right: Box::new(infer_expr(b, query_source_lookup, backend)?),
            op: String::from("IS NOT DISTINCT FROM"),
            nullability: OperatorNullability::NeverNull,
        }),
        Expr::Between {
            expr,
            negated,
            low,
            high,
        } => Ok(Expression::Between {
            left: Box::new(infer_expr(expr, query_source_lookup, backend)?),
            negated: *negated,
            low: Box::new(infer_expr(low, query_source_lookup, backend)?),
            high: Box::new(infer_expr(high, query_source_lookup, backend)?),
        }),
        Expr::SimilarTo {
            negated,
            expr,
            pattern,
            escape_char,
        } if escape_char.is_none() => {
            let op = if *negated {
                "NOT SIMILAR TO"
            } else {
                "SIMILAR TO"
            };
            Ok(Expression::BinaryOp {
                left: Box::new(infer_expr(expr, query_source_lookup, backend)?),
                right: Box::new(infer_expr(pattern, query_source_lookup, backend)?),
                op: String::from(op),
                nullability: pattern_nullability(backend),
            })
        }
        Expr::RLike {
            negated,
            expr,
            pattern,
            regexp,
        } => {
            let op = if *regexp {
                if *negated { "NOT REGEXP" } else { "REGEXP" }
            } else if *negated {
                "NOT RLIKE"
            } else {
                "RLIKE"
            };
            Ok(Expression::BinaryOp {
                left: Box::new(infer_expr(expr, query_source_lookup, backend)?),
                right: Box::new(infer_expr(pattern, query_source_lookup, backend)?),
                op: String::from(op),
                nullability: pattern_nullability(backend),
            })
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            let operand = operand
                .as_ref()
                .map(|o| infer_expr(o, query_source_lookup, backend))
                .transpose()?
                .map(Box::new);
            let else_clause = else_result
                .as_ref()
                .map(|e| infer_expr(e, query_source_lookup, backend))
                .transpose()?
                .map(Box::new);
            let conditions = conditions
                .iter()
                .map(|c| -> std::result::Result<_, _> {
                    let condition = infer_expr(&c.condition, query_source_lookup, backend)?;
                    let result = infer_expr(&c.result, query_source_lookup, backend)?;
                    Ok(CaseCondition { condition, result })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Expression::Case {
                operand,
                conditions,
                else_clause,
            })
        }
        Expr::InList {
            expr,
            list,
            negated,
        } => Ok(Expression::In {
            left: Box::new(infer_expr(expr, query_source_lookup, backend)?),
            negated: *negated,
            list: list
                .iter()
                .map(|e| infer_expr(e, query_source_lookup, backend))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        Expr::Nested(n) => infer_expr(n, query_source_lookup, backend)
            .map(Box::new)
            .map(Expression::Grouped),
        Expr::Subquery(query) => {
            let results = crate::select::parse_query(query, Some(query_source_lookup), backend)?;
            Ok(Expression::Subquery { selection: results })
        }
        // the resolver only knows the relations of the queries around a subquery, not
        // those the subquery defines itself
        Expr::InSubquery { subquery, .. } if defines_relations(subquery) => Ok(Expression::Unknown),
        Expr::InSubquery {
            expr,
            subquery,
            negated,
        } => {
            let results = crate::select::parse_query(subquery, Some(query_source_lookup), backend)?;
            Ok(Expression::InSubQuery {
                left: Box::new(infer_expr(expr, query_source_lookup, backend)?),
                negated: *negated,
                subquery: results,
            })
        }
        // other kinds of expressions still need to be supported
        Expr::CompoundFieldAccess { .. }
        | Expr::JsonAccess { .. }
        | Expr::IsFalse(..)
        | Expr::IsNotFalse(..)
        | Expr::IsTrue(..)
        | Expr::IsNotTrue(..)
        | Expr::IsUnknown(..)
        | Expr::IsNotUnknown(..)
        | Expr::IsNormalized { .. }
        | Expr::InUnnest { .. }
        | Expr::Like { .. }
        | Expr::ILike { .. }
        | Expr::SimilarTo { .. }
        | Expr::AnyOp { .. }
        | Expr::AllOp { .. }
        | Expr::UnaryOp { .. }
        | Expr::Convert { .. }
        | Expr::AtTimeZone { .. }
        | Expr::Extract { .. }
        | Expr::Ceil { .. }
        | Expr::Floor { .. }
        | Expr::Position { .. }
        | Expr::Substring { .. }
        | Expr::Trim { .. }
        | Expr::Overlay { .. }
        | Expr::Collate { .. }
        | Expr::Prefixed { .. }
        | Expr::TypedString(..)
        | Expr::Exists { .. }
        | Expr::GroupingSets(..)
        | Expr::Cube(..)
        | Expr::Rollup(..)
        | Expr::Tuple(..)
        | Expr::Struct { .. }
        | Expr::Named { .. }
        | Expr::Dictionary(..)
        | Expr::Map(..)
        | Expr::Array(..)
        | Expr::Interval(..)
        | Expr::MatchAgainst { .. }
        | Expr::Wildcard(..)
        | Expr::QualifiedWildcard(..)
        | Expr::OuterJoin(..)
        | Expr::Prior(..)
        | Expr::Lambda(..)
        | Expr::MemberOf(..)
        | Expr::IsJson { .. } => Ok(Expression::Unknown),
    }
}

/// How the result of `op` can be `NULL`
///
/// Only operators known to return `NULL` exactly for `NULL` operands are listed.
/// Anything else can return `NULL` for any operands: `/` and `%` for a zero divisor
/// in SQLite and MySQL, `->>` for a missing key, and even `+`, `-`, and `*`, as SQLite
/// turns a NaN result like `'1e999' - '1e999'` into `NULL` and PostgreSQL's `+` on
/// closed paths returns `NULL`. SQLite's `REGEXP` and `MATCH` call functions only an
/// application defines, see [`pattern_nullability`].
fn binary_operator_nullability(op: &BinaryOperator) -> OperatorNullability {
    use BinaryOperator::*;
    match op {
        // `^`, a power in PostgreSQL and an exclusive or in MySQL
        PGExp
        // comparison and logic
        | Eq | NotEq | Lt | LtEq | Gt | GtEq | Spaceship | And | Or | Xor
        // strings and pattern matching
        | StringConcat | PGRegexMatch | PGRegexIMatch | PGRegexNotMatch
        | PGRegexNotIMatch | PGLikeMatch | PGILikeMatch | PGNotLikeMatch | PGNotILikeMatch
        | PGStartsWith
        // bits, without PostgreSQL's `#`, which also intersects geometric shapes
        | BitwiseOr | BitwiseAnd | BitwiseXor | PGBitwiseShiftLeft | PGBitwiseShiftRight
        // PostgreSQL containment and key checks
        | PGOverlap | AtArrow | ArrowAt | HashMinus | Question | QuestionAnd | QuestionPipe => {
            OperatorNullability::NullIfOperandNull
        }
        _ => OperatorNullability::Nullable,
    }
}

/// How the result of a pattern match like `LIKE` can be `NULL`
///
/// SQLite evaluates `LIKE` with a function an application can replace, and `REGEXP`
/// with one only an application defines, and such a function can return `NULL` for any
/// operands.
fn pattern_nullability(backend: Backend) -> OperatorNullability {
    match backend {
        Backend::Sqlite => OperatorNullability::Nullable,
        Backend::Pg | Backend::Mysql | Backend::Mariadb => OperatorNullability::NullIfOperandNull,
    }
}

/// Whether every operator in `expr` binds tighter than `IS`
///
/// Only then does the right operand sqlparser finds for `IS [NOT] DISTINCT FROM`
/// match the database's. The listed operators bind tighter in SQLite, and PostgreSQL
/// binds even its comparisons tighter. sqlparser ranks some operators differently, for
/// example `->>` below `=`, so the whole expression counts, except for parenthesized
/// parts and the arguments of calls, casts, and `CASE`. Expressions not listed count as
/// binding looser.
fn binds_tighter_than_is(expr: &Expr) -> bool {
    use BinaryOperator::*;
    match expr {
        Expr::BinaryOp { left, op, right } => {
            matches!(
                op,
                Plus | Minus
                    | Multiply
                    | Divide
                    | Modulo
                    | StringConcat
                    | Arrow
                    | LongArrow
                    | BitwiseAnd
                    | BitwiseOr
                    | PGBitwiseShiftLeft
                    | PGBitwiseShiftRight
                    | Lt
                    | LtEq
                    | Gt
                    | GtEq
            ) && binds_tighter_than_is(left)
                && binds_tighter_than_is(right)
        }
        Expr::UnaryOp { op, expr } => {
            !matches!(op, UnaryOperator::Not) && binds_tighter_than_is(expr)
        }
        Expr::Value(_)
        | Expr::Identifier(_)
        | Expr::CompoundIdentifier(_)
        | Expr::Function(_)
        | Expr::Cast { .. }
        | Expr::Case { .. }
        | Expr::Nested(_)
        | Expr::Subquery(_) => true,
        _ => false,
    }
}

/// Does `query` define a common table expression or derived table anywhere?
fn defines_relations(query: &sqlparser::ast::Query) -> bool {
    struct Definitions;

    impl Visitor for Definitions {
        type Break = ();

        fn pre_visit_query(&mut self, query: &sqlparser::ast::Query) -> ControlFlow<()> {
            if query.with.is_some() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }

        fn pre_visit_table_factor(&mut self, factor: &TableFactor) -> ControlFlow<()> {
            if matches!(factor, TableFactor::Derived { .. }) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
    }

    query.visit(&mut Definitions).is_break()
}

fn infer_functions(
    f: &sqlparser::ast::Function,
    query_source_lookup: &HashMap<Option<&str>, QuerySource<'_>>,
    backend: Backend,
) -> Result<Expression> {
    let (name, schema) = match f.name.0.as_slice() {
        [ObjectNamePart::Identifier(name)] => (&name.value, None),
        [
            ObjectNamePart::Identifier(schema),
            ObjectNamePart::Identifier(name),
        ] => (&name.value, Some(&schema.value)),
        _ => return Ok(Expression::Unknown),
    };
    let args = match &f.args {
        FunctionArguments::List(l) if l.duplicate_treatment.is_none() && l.clauses.is_empty() => l
            .args
            .iter()
            .map(|arg| match arg {
                FunctionArg::Named { arg, .. }
                | FunctionArg::ExprNamed { arg, .. }
                | FunctionArg::Unnamed(arg) => match arg {
                    FunctionArgExpr::Expr(expr) => infer_expr(expr, query_source_lookup, backend),
                    FunctionArgExpr::QualifiedWildcard(object_name) => {
                        if let Some(item) = object_name
                            .0
                            .last()
                            .and_then(|a| a.as_ident())
                            .map(|a| a.value.as_str())
                            .and_then(|k| find_query_source(query_source_lookup, k))
                        {
                            let nullable_row = item.contains_left_join(query_source_lookup)?;
                            Ok(Expression::Wildcard {
                                nullable_row,
                                relation: item.name.map(|s| s.to_owned()),
                                schema: item.schema.map(|t| t.to_owned()),
                            })
                        } else {
                            Ok(Expression::Unknown)
                        }
                    }
                    FunctionArgExpr::Wildcard if query_source_lookup.len() == 1 => {
                        let query_source_lookup = query_source_lookup
                            .values()
                            .next()
                            .expect("We have exactly one element");
                        Ok(Expression::Wildcard {
                            nullable_row: false,
                            relation: query_source_lookup.name.map(|s| s.to_owned()),
                            schema: query_source_lookup.schema.map(|t| t.to_owned()),
                        })
                    }
                    FunctionArgExpr::WildcardWithOptions(_) | FunctionArgExpr::Wildcard => {
                        Ok(Expression::Unknown)
                    }
                },
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Ok(Expression::Unknown),
    };
    Ok(Expression::Function {
        name: name.clone(),
        schema: schema.cloned(),
        arguments: args,
    })
}
