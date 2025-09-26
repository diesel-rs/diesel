// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::SchemaResolver;
use crate::error::Error;
use crate::error::Result;
use crate::query_source::QuerySource;
use sqlparser::ast::SelectItem;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub(crate) struct SelectField {
    pub(crate) ident: Option<String>,
    pub(crate) kind: SelectFieldKind,
}

impl SelectField {
    pub(crate) fn infer_nullability(
        &self,
        resolver: &mut (dyn SchemaResolver + '_),
    ) -> Result<Option<bool>> {
        self.kind.infer_nullability(resolver)
    }
}

/// Different kind of expressions in a SELECT clause
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum SelectFieldKind {
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
        inner: Box<SelectFieldKind>,
        /// target cast type
        tpe: String,
    },
    /// A unknown select expression
    Unknown,
}

impl SelectFieldKind {
    pub(crate) fn infer_nullability(
        &self,
        resolver: &mut (dyn SchemaResolver + '_),
    ) -> Result<Option<bool>> {
        match self {
            // literals are null for literal `NULL` values
            // Otherwise they cannot be null
            SelectFieldKind::Literal { is_null, .. } => Ok(Some(*is_null)),
            // If the field comes in via an `LEFT JOIN`
            // we unconditionally assume it is nullable
            SelectFieldKind::Field {
                via_left_join: true,
                ..
            } => Ok(Some(true)),
            // Otherwise we ask the schema resolver for  more information about
            // this particular field
            SelectFieldKind::Field {
                schema,
                query_source: table,
                field_name,
                ..
            } => Ok(Some(
                resolver
                    .resolve_field(schema.as_deref(), table, field_name)
                    .map_err(Error::ResolverFailure)?
                    .is_nullable(),
            )),
            // for casts we assume that they return `NULL` only if the inner expression is `NULL`
            SelectFieldKind::Cast { inner, .. } => inner.infer_nullability(resolver),
            // unknown is not known, so return None
            SelectFieldKind::Unknown => Ok(None),
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
        s @ SelectItem::QualifiedWildcard(
            _select_item_qualified_wildcard_kind,
            _wildcard_additional_options,
        ) => Err(Error::UnsupportedSql {
            msg: format!("Unsupported `SELECT` expression: `{s}`"),
        }),
        s @ SelectItem::Wildcard(_wildcard_additional_options) => Err(Error::UnsupportedSql {
            msg: format!("Unsupported `SELECT` expression: `{s}`"),
        }),
    }
}
