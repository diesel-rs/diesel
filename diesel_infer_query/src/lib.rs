// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A crate to infer details about SQL queries
//! This crate currently supports inferring whether a fields of a
//! view are nullable or not

#![warn(missing_docs)]
mod backend;
mod error;
mod expression;
mod functions;
mod query_source;
mod resolver;
mod select;
mod views;

#[doc(inline)]
pub use crate::backend::Backend;
#[doc(inline)]
pub use crate::error::{Error, Result};
#[doc(inline)]
pub use crate::resolver::{SchemaField, SchemaResolver};
#[doc(inline)]
pub use crate::views::{ViewData, parse_view_def};

/// Indicates if a certain expression is nullable or not
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsNull {
    /// The expression might produce a `NULL` value
    IsNullable,
    /// The expression cannot produce a `NULL` values
    NotNullable,
    /// It's unknown whether or not the expression
    /// can produce a `NULL` value
    Unknown,
}

impl IsNull {
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::IsNullable, _) | (_, Self::IsNullable) => Self::IsNullable,
            (Self::NotNullable, Self::NotNullable) => Self::NotNullable,
        }
    }
}
