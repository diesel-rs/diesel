// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::error::{Error, Result};
use crate::query_source::QuerySource;
use crate::select::SelectFieldKind;
use sqlparser::ast::{Expr, Value};
use std::collections::HashMap;

pub(crate) fn infer_expr(
    expr: &sqlparser::ast::Expr,
    query_source_lookup: &HashMap<&str, QuerySource>,
) -> Result<SelectFieldKind> {
    match expr {
        Expr::Value(v) => Ok(SelectFieldKind::Literal {
            value: v.value.clone().into_string(),
            is_null: matches!(v.value, Value::Null),
        }),
        Expr::Identifier(id) => {
            if query_source_lookup.len() == 1 {
                let table = query_source_lookup
                    .values()
                    .next()
                    .expect("We checked there is only one value");
                Ok(SelectFieldKind::Field {
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
                let via_left_join = table.contains_left_join(&query_source_lookup)?;
                Ok(SelectFieldKind::Field {
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
            Ok(SelectFieldKind::Cast {
                inner: Box::new(inner),
                tpe: data_type.to_string(),
            })
        }
        // other kinds of expressions still need to be supported
        _e => Ok(SelectFieldKind::Unknown),
    }
}
