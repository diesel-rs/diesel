// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::Result;
use crate::select::SelectField;
use crate::views::SubQuery;
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

pub(crate) struct CombinedResolver<'b> {
    subqueries: Vec<Vec<ResolvedField>>,
    fallback: &'b mut dyn SchemaResolver,
}

impl<'b> CombinedResolver<'b> {
    pub(crate) fn empty(capacity: usize, fallback: &'b mut dyn SchemaResolver) -> Self {
        Self {
            fallback,
            subqueries: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn new(
        subqueries: &[SubQuery],
        fallback: &'b mut dyn SchemaResolver,
    ) -> Result<Self> {
        let mut resolver = Self::empty(subqueries.len(), fallback);
        for subquery in subqueries {
            resolver.insert(&subquery.fields)?;
        }
        Ok(resolver)
    }

    pub(crate) fn insert(&mut self, fields: &[SelectField]) -> Result<()> {
        let fields = fields
            .iter()
            .map(|field| {
                Ok(ResolvedField {
                    is_null: field.infer_nullability(self)?,
                    ident: field.ident.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.subqueries.push(fields);
        Ok(())
    }

    pub(crate) fn resolve_derived_field(
        &self,
        source_id: usize,
        relation: Option<&str>,
        field_name: &str,
    ) -> Result<&dyn SchemaField> {
        self.subqueries
            .get(source_id)
            .and_then(|fields| {
                fields
                    .iter()
                    .find(|field| field.ident.as_deref() == Some(field_name))
            })
            .map(|field| field as &dyn SchemaField)
            .ok_or_else(|| Error::UnknownField {
                relation_schema: None,
                query_relation: relation.map(str::to_owned),
                field_name: field_name.to_owned(),
            })
    }

    pub(crate) fn list_derived_fields(&self, source_id: usize) -> Result<Vec<&dyn SchemaField>> {
        self.subqueries
            .get(source_id)
            .map(|fields| {
                fields
                    .iter()
                    .map(|field| field as &dyn SchemaField)
                    .collect()
            })
            .ok_or_else(|| Error::UnsupportedSql {
                msg: "Unresolved subquery".to_owned(),
            })
    }
}

impl SchemaResolver for CombinedResolver<'_> {
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
        field_name: &str,
    ) -> Result<&'s dyn SchemaField, Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.fallback
            .resolve_field(relation_schema, query_relation, field_name)
    }

    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.fallback.list_fields(relation_schema, query_relation)
    }
}

#[derive(Debug)]
struct ResolvedField {
    ident: Option<String>,
    is_null: IsNull,
}

impl SchemaField for ResolvedField {
    fn is_nullable(&self) -> IsNull {
        self.is_null
    }

    fn name(&self) -> Option<&str> {
        self.ident.as_deref()
    }
}
