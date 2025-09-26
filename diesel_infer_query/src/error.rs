// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Different kinds of errors returned by this crate
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Parsing the SQL failed with the provided error message
    #[error("Failed to parse sql: {0:?}")]
    ParserError(#[from] sqlparser::parser::ParserError),
    /// Handling this kind of SQL is currently not supported by this crate
    #[error("Unsupported SQL: {msg}")]
    UnsupportedSql {
        /// details about the unsupported SQL expression
        msg: String,
    },
    /// The query referenced a unknown query source
    #[error("Querysource was not found in the from clause: `{query_source}`")]
    InvalidQuerySource {
        /// Which query source is unknown
        query_source: String,
    },
    /// The schema resolver returned an error
    #[error("Could not resolve view data: {0}")]
    ResolverFailure(Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// A result type using the error provided by this crate as default
pub type Result<T, E = Error> = std::result::Result<T, E>;
