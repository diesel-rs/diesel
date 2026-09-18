// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Different kinds of errors returned by this crate
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Parsing the SQL failed with the provided error message
    #[error("failed to parse sql: {0:?}")]
    #[non_exhaustive]
    ParserError(#[from] sqlparser::parser::ParserError),
    /// Handling this kind of SQL is currently not supported by this crate
    #[error("unsupported SQL: {msg}")]
    UnsupportedSql {
        /// details about the unsupported SQL expression
        msg: String,
    },
    /// The query referenced a unknown query source
    #[error("query source was not found in the from clause: `{query_source}`")]
    #[non_exhaustive]
    InvalidQuerySource {
        /// Which query source is unknown
        query_source: String,
    },
    /// The schema resolver returned an error
    #[error("could not resolve view data: {inner}")]
    #[non_exhaustive]
    ResolverFailure {
        /// The inner resolver failure
        inner: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    /// The query contained an unnamed field that needs to be resolved
    #[error("could not get a field name for an expression")]
    UnnamedField,
    #[error(
        "could not find a field with the following name: `{relation_schema:?}.{query_relation:?}.{field_name}`"
    )]
    /// The query referenced an unknown field
    #[non_exhaustive]
    UnknownField {
        /// The schema of the referenced field
        relation_schema: Option<String>,
        /// The query relation of the referenced field
        query_relation: Option<String>,
        /// The field name of the referenced field
        field_name: String,
    },
    /// We found an unresolved wildcard expression while inferring nullability
    ///
    /// Make sure to call `resolve_references()` before calling `infer_nullability()`
    #[error("found an unresolved wild card expression")]
    UnresolvedWildcard,
}

/// A result type using the error provided by this crate as default
pub type Result<T, E = Error> = std::result::Result<T, E>;
