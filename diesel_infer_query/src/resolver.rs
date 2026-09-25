// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::Result;
use crate::query_source::only_match_ignoring_case;
use crate::views::{SubQuery, unknown_fields};
use crate::{Error, IsNull};

/// A generic interface that allows this crate
/// to request more information about certain database
/// relations
pub trait SchemaResolver {
    /// Resolve a specific database field
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
        field_name: &str,
    ) -> Result<&'s dyn SchemaField, Box<dyn std::error::Error + Send + Sync + 'static>>;

    /// Get a list of fields for a specific database relation
    /// in the order returned by the database
    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn std::error::Error + Send + Sync + 'static>>;
}

/// A generic representation of a database field
pub trait SchemaField {
    /// Is this field nullable
    fn is_nullable(&self) -> IsNull;
    /// The database side name of the field
    fn name(&self) -> Option<&str>;
}

/// Resolves the common table expressions and derived tables of a view definition in
/// their scope, and every other relation with the resolver of the database
pub(crate) struct CombinedResolver<'b> {
    /// the common table expressions and derived tables in scope, innermost last
    subqueries: Vec<(Option<String>, Vec<ResolvedField>)>,
    fallback: &'b mut dyn SchemaResolver,
}

/// A scope entered by [`CombinedResolver::enter`]
pub(crate) struct Scope(usize);

impl<'b> CombinedResolver<'b> {
    pub(crate) fn new(fallback: &'b mut dyn SchemaResolver) -> Self {
        Self {
            subqueries: Vec::new(),
            fallback,
        }
    }

    /// Enter the scope of `subquery`, which is called `name`
    ///
    /// A recursive common table expression reads itself, and until its nullability is
    /// inferred, it is unknown.
    pub(crate) fn enter(&mut self, name: &Option<String>, subquery: &SubQuery) -> Scope {
        let scope = Scope(self.subqueries.len());
        if subquery.recursive() {
            self.define(name.clone(), unknown_fields(subquery.fields()));
        }
        scope
    }

    /// Leave `scope`, forgetting what was defined in it
    pub(crate) fn leave(&mut self, scope: Scope) {
        self.subqueries.truncate(scope.0);
    }

    /// Define the relation `name` with `fields` in the current scope
    pub(crate) fn define(&mut self, name: Option<String>, fields: Vec<ResolvedField>) {
        self.subqueries.push((name, fields));
    }
}

/// The fields of the common table expression or derived table named `name`
///
/// The innermost one wins, and names are looked up like
/// [`crate::query_source::find_query_source`] looks up query sources. A name that
/// only matches an inner relation ignoring ASCII case, but an outer one exactly, is
/// ambiguous.
fn find_subquery<'s>(
    subqueries: &'s [(Option<String>, Vec<ResolvedField>)],
    name: Option<&str>,
) -> Result<Option<&'s [ResolvedField]>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let Some(name) = name else {
        return Ok(subqueries
            .iter()
            .rev()
            .find(|(other, _)| other.is_none())
            .map(|(_, fields)| fields.as_slice()));
    };
    let mut matching = subqueries.iter().rev().filter(|(other, _)| {
        other
            .as_deref()
            .is_some_and(|other| other.eq_ignore_ascii_case(name))
    });
    let Some((innermost, fields)) = matching.next() else {
        return Ok(None);
    };
    if innermost.as_deref() != Some(name)
        && matching.any(|(other, _)| other.as_deref() == Some(name))
    {
        return Err(Box::new(Error::UnsupportedSql {
            msg: format!("Ambiguous relation `{name}`"),
        }));
    }
    Ok(Some(fields))
}

impl<'b> SchemaResolver for CombinedResolver<'b> {
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
        field_name: &str,
    ) -> Result<&'s dyn SchemaField, Box<dyn std::error::Error + Send + Sync + 'static>> {
        if relation_schema.is_none()
            && let Some(fields) = find_subquery(&self.subqueries, query_relation)?
        {
            fields
                .iter()
                .find(|f| f.ident.as_deref() == Some(field_name))
                .or_else(|| {
                    only_match_ignoring_case(
                        fields.iter().filter_map(|f| Some((f.ident.as_deref()?, f))),
                        field_name,
                    )
                })
                .map(|f| f as &dyn SchemaField)
                .ok_or_else(|| {
                    Box::new(Error::UnknownField {
                        relation_schema: relation_schema.map(|c| c.to_owned()),
                        query_relation: query_relation.map(|c| c.to_owned()),
                        field_name: field_name.to_owned(),
                    })
                    .into()
                })
        } else {
            self.fallback
                .resolve_field(relation_schema, query_relation, field_name)
        }
    }

    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        if relation_schema.is_none()
            && let Some(fields) = find_subquery(&self.subqueries, query_relation)?
        {
            Ok(fields.iter().map(|f| f as &dyn SchemaField).collect())
        } else {
            self.fallback.list_fields(relation_schema, query_relation)
        }
    }
}

#[derive(Debug)]
pub(crate) struct ResolvedField {
    ident: Option<String>,
    is_null: IsNull,
}

impl ResolvedField {
    pub(crate) fn new(ident: Option<String>, is_null: IsNull) -> Self {
        Self { ident, is_null }
    }
}

impl SchemaField for ResolvedField {
    fn is_nullable(&self) -> IsNull {
        self.is_null
    }

    fn name(&self) -> Option<&str> {
        self.ident.as_deref()
    }
}
