#![recursion_limit = "1024"]
// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(needless_pass_by_value))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
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

extern crate diesel;
#[cfg(feature = "diesel_infer_schema")]
extern crate diesel_infer_schema;
#[cfg(all(feature = "dotenv", feature = "diesel_infer_schema"))]
extern crate dotenv;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

mod as_changeset;
mod associations;
mod ast_builder;
mod attr;
mod embed_migrations;
mod identifiable;
mod insertable;
mod model;
mod queryable;
mod queryable_by_name;
#[cfg(feature = "diesel_infer_schema")]
mod schema_inference;
#[cfg(feature = "diesel_infer_schema")]
mod database_url;
mod util;
mod migrations;

use proc_macro::TokenStream;
use syn::parse_derive_input;

#[proc_macro_derive(Queryable, attributes(column_name))]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_derive(input, queryable::derive_queryable)
}

#[proc_macro_derive(QueryableByName, attributes(table_name, column_name))]
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

#[proc_macro_derive(InferSchema, attributes(infer_schema_options))]
#[cfg(feature = "diesel_infer_schema")]
pub fn derive_infer_schema(input: TokenStream) -> TokenStream {
    expand_derive(input, schema_inference::derive_infer_schema)
}

#[proc_macro_derive(InferTableFromSchema, attributes(infer_table_from_schema_options))]
#[cfg(feature = "diesel_infer_schema")]
pub fn derive_infer_table_from_schema(input: TokenStream) -> TokenStream {
    expand_derive(input, schema_inference::derive_infer_table_from_schema)
}

#[proc_macro_derive(EmbedMigrations, attributes(embed_migrations_options))]
pub fn derive_embed_migrations(input: TokenStream) -> TokenStream {
    expand_derive(input, embed_migrations::derive_embed_migrations)
}

fn expand_derive(input: TokenStream, f: fn(syn::DeriveInput) -> quote::Tokens) -> TokenStream {
    let item = parse_derive_input(&input.to_string()).unwrap();
    f(item).to_string().parse().unwrap()
}
