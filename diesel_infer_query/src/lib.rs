// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A crate to infer details about SQL queries
//! This crate currently supports inferring whether a fields of a
//! view are nullable or not

#![warn(missing_docs)]
mod error;
mod expression;
mod query_source;
mod resolver;
mod select;
mod views;

#[doc(inline)]
pub use crate::error::{Error, Result};
#[doc(inline)]
pub use crate::resolver::{SchemaField, SchemaResolver};
#[doc(inline)]
pub use crate::views::{ViewData, parse_view_def};
