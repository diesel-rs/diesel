// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
}

/// A generic representation of a database field
pub trait SchemaField {
    /// Is this field nullable
    fn is_nullable(&self) -> bool;
}
