#![recursion_limit = "1024"]
// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![allow(
    clippy::needless_pass_by_value,
    clippy::option_map_unwrap_or_else,
    clippy::option_map_unwrap_or
)]
#![warn(
    clippy::wrong_pub_self_convention,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic, proc_macro_span))]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

mod diagnostic_shim;
mod field;
mod meta;
mod model;
mod resolved_at_shim;
mod util;

mod as_changeset;
mod as_expression;
mod associations;
mod diesel_numeric_ops;
mod from_sql_row;
mod identifiable;
mod insertable;
mod non_aggregate;
mod query_id;
mod queryable;
mod queryable_by_name;
mod sql_function;
mod sql_type;

use diagnostic_shim::*;

#[proc_macro_derive(
    AsChangeset,
    attributes(table_name, primary_key, column_name, changeset_options)
)]
pub fn derive_as_changeset(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, as_changeset::derive)
}

#[proc_macro_derive(AsExpression, attributes(diesel, sql_type))]
pub fn derive_as_expression(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, as_expression::derive)
}

#[proc_macro_derive(Associations, attributes(belongs_to, column_name, table_name))]
pub fn derive_associations(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, associations::derive)
}

#[proc_macro_derive(DieselNumericOps)]
pub fn derive_diesel_numeric_ops(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, diesel_numeric_ops::derive)
}

#[proc_macro_derive(FromSqlRow, attributes(diesel))]
pub fn derive_from_sql_row(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, from_sql_row::derive)
}

#[proc_macro_derive(Identifiable, attributes(table_name, primary_key, column_name))]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, identifiable::derive)
}

#[proc_macro_derive(Insertable, attributes(table_name, column_name, diesel))]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, insertable::derive)
}

#[proc_macro_derive(NonAggregate)]
pub fn derive_non_aggregate(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, non_aggregate::derive)
}

#[proc_macro_derive(QueryId)]
pub fn derive_query_id(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, query_id::derive)
}

#[proc_macro_derive(Queryable, attributes(column_name, diesel))]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, queryable::derive)
}

#[proc_macro_derive(QueryableByName, attributes(table_name, column_name, sql_type, diesel))]
pub fn derive_queryable_by_name(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, queryable_by_name::derive)
}

#[proc_macro_derive(SqlType, attributes(postgres, sqlite_type, mysql_type))]
pub fn derive_sql_type(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, sql_type::derive)
}

#[proc_macro]
pub fn sql_function_proc(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, sql_function::expand)
}

fn expand_proc_macro<T: syn::parse::Parse>(
    input: TokenStream,
    f: fn(T) -> Result<proc_macro2::TokenStream, Diagnostic>,
) -> TokenStream {
    let item = syn::parse(input).unwrap();
    match f(item) {
        Ok(x) => x.into(),
        Err(e) => {
            e.emit();
            "".parse().unwrap()
        }
    }
}
