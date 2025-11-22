// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::Result;
use crate::views::SubQuery;
use crate::{Error, IsNull};
use std::collections::HashMap;

/// A generic interface that allows this crate
/// to request more information about certain database
/// relations
pub trait SchemaResolver {
    /// Resolve a specific database field
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: &str,
        field_name: &str,
    ) -> Result<&'s dyn SchemaField, Box<dyn std::error::Error + Send + Sync + 'static>>;

    /// Get a list of fields for a specific database relation
    /// in the order returned by the database
    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: &str,
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
    subqueries: HashMap<String, Vec<ResolvedField>>,
    fallback: &'b mut dyn SchemaResolver,
}

impl<'b> CombinedResolver<'b> {
    pub(crate) fn new(
        subqueries: &[(String, SubQuery)],
        fallback: &'b mut dyn SchemaResolver,
    ) -> Result<Self> {
        let mut resolver = Self {
            fallback,
            subqueries: HashMap::new(),
        };
        for (k, s) in subqueries {
            let fields = s
                .fields()
                .iter()
                .map(|f| {
                    let is_null = f.infer_nullability(&mut resolver)?;
                    Ok(ResolvedField {
                        is_null,
                        ident: f.ident.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            resolver.subqueries.insert(k.to_owned(), fields);
        }
        Ok(resolver)
    }
}

impl<'b> SchemaResolver for CombinedResolver<'b> {
    fn resolve_field<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: &str,
        field_name: &str,
    ) -> Result<&'s dyn SchemaField, Box<dyn std::error::Error + Send + Sync + 'static>> {
        if relation_schema.is_none()
            && let Some(fields) = self.subqueries.get(query_relation)
        {
            fields
                .iter()
                .find_map(|f| {
                    if f.ident.as_deref() == Some(field_name) {
                        Some(f as &dyn SchemaField)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    Box::new(Error::UnknownField {
                        relation_schema: relation_schema.map(|c| c.to_owned()),
                        query_relation: query_relation.to_owned(),
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
        query_relation: &str,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        if relation_schema.is_none()
            && let Some(fields) = self.subqueries.get(query_relation)
        {
            Ok(fields.iter().map(|f| f as &dyn SchemaField).collect())
        } else {
            self.fallback.list_fields(relation_schema, query_relation)
        }
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
