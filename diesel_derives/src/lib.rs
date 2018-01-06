#![recursion_limit = "1024"]
// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(needless_pass_by_value))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "clippy", allow(option_map_unwrap_or_else, option_map_unwrap_or))]
#![cfg_attr(feature = "clippy",
            warn(wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, if_not_else, items_after_statements, used_underscore_binding))]

macro_rules! t {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("{}", e),
        }
    };
}

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

mod as_changeset;
mod as_expression;
mod associations;
mod ast_builder;
mod attr;
mod from_sql_row;
mod identifiable;
mod insertable;
mod model;
mod query_id;
mod queryable;
mod queryable_by_name;
mod util;

use proc_macro::TokenStream;
use syn::parse_derive_input;

#[proc_macro_derive(Queryable, attributes(column_name))]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_derive(input, queryable::derive_queryable)
}

#[proc_macro_derive(QueryableByName, attributes(table_name, column_name, sql_type, diesel))]
pub fn derive_queryable_by_name(input: TokenStream) -> TokenStream {
    expand_derive(input, queryable_by_name::derive)
}

#[proc_macro_derive(Identifiable, attributes(table_name, primary_key))]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    expand_derive(input, identifiable::derive_identifiable)
}

#[proc_macro_derive(Insertable, attributes(table_name, column_name))]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_derive(input, insertable::derive_insertable)
}

#[proc_macro_derive(AsChangeset,
                    attributes(table_name, primary_key, column_name, changeset_options))]
pub fn derive_as_changeset(input: TokenStream) -> TokenStream {
    expand_derive(input, as_changeset::derive_as_changeset)
}

#[proc_macro_derive(Associations, attributes(table_name, belongs_to))]
pub fn derive_associations(input: TokenStream) -> TokenStream {
    expand_derive(input, associations::derive_associations)
}

#[proc_macro_derive(QueryId)]
pub fn derive_query_id(input: TokenStream) -> TokenStream {
    expand_derive(input, query_id::derive)
}

#[proc_macro_derive(FromSqlRow, attributes(diesel))]
pub fn derive_from_sql_row(input: TokenStream) -> TokenStream {
    expand_derive(input, from_sql_row::derive)
}

#[proc_macro_derive(AsExpression, attributes(diesel, sql_type))]
pub fn derive_from_as_expression(input: TokenStream) -> TokenStream {
    expand_derive(input, as_expression::derive)
}

fn expand_derive(input: TokenStream, f: fn(syn::DeriveInput) -> quote::Tokens) -> TokenStream {
    let item = parse_derive_input(&input.to_string()).unwrap();
    f(item).to_string().parse().unwrap()
}
