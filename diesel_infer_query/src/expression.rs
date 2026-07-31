// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::{Error, Result};
use crate::query_source::QuerySource;
use crate::select::{CaseCondition, Expression};
use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, FunctionArguments, ObjectNamePart, Value,
};
use std::collections::HashMap;

pub(crate) fn infer_expr(
    expr: &sqlparser::ast::Expr,
    query_source_lookup: &HashMap<&str, QuerySource>,
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
                    query_source: table.name.to_owned(),
                    field_name: id.value.clone(),
                    // no joins here so we should be fine
                    via_left_join: false,
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
                let table = query_source_lookup
                    .get(table.value.as_str())
                    .ok_or_else(|| Error::InvalidQuerySource {
                        query_source: table.value.clone(),
                    })?;
                // lookup if this field comes in via a `LEFT JOIN` somewhere in the chain
                let via_left_join = table.contains_left_join(query_source_lookup)?;
                Ok(Expression::Field {
                    schema: table.schema.map(|s| s.to_owned()),
                    query_source: table.name.to_owned(),
                    field_name: field.value.clone(),
                    via_left_join,
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
            let inner = infer_expr(expr, query_source_lookup)?;
            Ok(Expression::Cast {
                inner: Box::new(inner),
                tpe: data_type.to_string(),
            })
        }
        Expr::BinaryOp { left, op, right } => {
            let left = Box::new(infer_expr(left, query_source_lookup)?);
            let right = Box::new(infer_expr(right, query_source_lookup)?);
            Ok(Expression::BinaryOp {
                left,
                right,
                op: op.to_string(),
                statically_not_null: false,
            })
        }
        Expr::IsNull(e) => {
            let inner = Box::new(infer_expr(e, query_source_lookup)?);
            Ok(Expression::PostfixOp {
                expr: inner,
                op: String::from("IS NULL"),
                statically_not_null: true,
            })
        }
        Expr::IsNotNull(e) => {
            let inner = Box::new(infer_expr(e, query_source_lookup)?);
            Ok(Expression::PostfixOp {
                expr: inner,
                op: String::from("IS NOT NULL"),
                statically_not_null: true,
            })
        }
        Expr::Function(f) => infer_functions(f, query_source_lookup),
        Expr::Like {
            negated,
            any,
            expr,
            pattern,
            escape_char,
        } if !*any && escape_char.is_none() => {
            let op = if *negated { "NOT LIKE" } else { "LIKE" };
            Ok(Expression::BinaryOp {
                left: Box::new(infer_expr(expr, query_source_lookup)?),
                right: Box::new(infer_expr(pattern, query_source_lookup)?),
                op: String::from(op),
                statically_not_null: false,
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
                left: Box::new(infer_expr(expr, query_source_lookup)?),
                right: Box::new(infer_expr(pattern, query_source_lookup)?),
                op: String::from(op),
                statically_not_null: false,
            })
        }
        Expr::IsDistinctFrom(a, b) => Ok(Expression::BinaryOp {
            left: Box::new(infer_expr(a, query_source_lookup)?),
            right: Box::new(infer_expr(b, query_source_lookup)?),
            op: String::from("IS DISTINCT FROM"),
            statically_not_null: true,
        }),
        Expr::IsNotDistinctFrom(a, b) => Ok(Expression::BinaryOp {
            left: Box::new(infer_expr(a, query_source_lookup)?),
            right: Box::new(infer_expr(b, query_source_lookup)?),
            op: String::from("IS NOT DISTINCT FROM"),
            statically_not_null: true,
        }),
        Expr::Between {
            expr,
            negated,
            low,
            high,
        } => Ok(Expression::Between {
            left: Box::new(infer_expr(expr, query_source_lookup)?),
            negated: *negated,
            low: Box::new(infer_expr(low, query_source_lookup)?),
            high: Box::new(infer_expr(high, query_source_lookup)?),
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
                left: Box::new(infer_expr(expr, query_source_lookup)?),
                right: Box::new(infer_expr(pattern, query_source_lookup)?),
                op: String::from(op),
                statically_not_null: false,
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
                left: Box::new(infer_expr(expr, query_source_lookup)?),
                right: Box::new(infer_expr(pattern, query_source_lookup)?),
                op: String::from(op),
                statically_not_null: false,
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
                .map(|o| infer_expr(o, query_source_lookup))
                .transpose()?
                .map(Box::new);
            let else_clause = else_result
                .as_ref()
                .map(|e| infer_expr(e, query_source_lookup))
                .transpose()?
                .map(Box::new);
            let conditions = conditions
                .iter()
                .map(|c| -> std::result::Result<_, _> {
                    let condition = infer_expr(&c.condition, query_source_lookup)?;
                    let result = infer_expr(&c.result, query_source_lookup)?;
                    Ok(CaseCondition { condition, result })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Expression::Case {
                operand,
                conditions,
                else_clause,
            })
        }
        // other kinds of expressions still need to be supported
        _e => {
            dbg!(_e);
            Ok(Expression::Unknown)
        }
    }
}

fn infer_functions(
    f: &sqlparser::ast::Function,
    query_source_lookup: &HashMap<&str, QuerySource<'_>>,
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
                    FunctionArgExpr::Expr(expr) => infer_expr(expr, query_source_lookup),
                    FunctionArgExpr::QualifiedWildcard(object_name) => {
                        if let Some(item) = object_name
                            .0
                            .last()
                            .and_then(|a| a.as_ident())
                            .map(|a| a.value.as_str())
                            .and_then(|k| query_source_lookup.get(k))
                        {
                            let is_left_joined = item.contains_left_join(query_source_lookup)?;
                            Ok(Expression::Wildcard {
                                is_left_joined,
                                relation: item.name.to_string(),
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
                            is_left_joined: false,
                            relation: query_source_lookup.name.to_string(),
                            schema: query_source_lookup.schema.map(|t| t.to_owned()),
                        })
                    }
                    FunctionArgExpr::Wildcard => Ok(Expression::Unknown),
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
