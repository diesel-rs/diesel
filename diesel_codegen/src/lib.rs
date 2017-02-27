#![deny(warnings)]

macro_rules! t {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("{}", e),
        }
    };
}

extern crate diesel_codegen_shared;
extern crate diesel_infer_schema;
extern crate diesel;
#[macro_use]
extern crate quote;
extern crate proc_macro;
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
#[cfg(any(feature = "postgres", feature = "sqlite"))]
mod schema_inference;
mod util;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(Queryable)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_derive(input, queryable::derive_queryable)
}

#[proc_macro_derive(Identifiable, attributes(table_name, primary_key))]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    expand_derive(input, identifiable::derive_identifiable)
}

#[proc_macro_derive(Insertable, attributes(table_name, column_name))]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_derive(input, insertable::derive_insertable)
}

#[proc_macro_derive(AsChangeset, attributes(table_name, column_name, changeset_options))]
pub fn derive_as_changeset(input: TokenStream) -> TokenStream {
    expand_derive(input, as_changeset::derive_as_changeset)
}

#[proc_macro_derive(Associations, attributes(table_name, has_many, belongs_to))]
pub fn derive_associations(input: TokenStream) -> TokenStream {
    expand_derive(input, associations::derive_associations)
}

#[proc_macro_derive(InferSchema, attributes(infer_schema_options))]
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub fn derive_infer_schema(input: TokenStream) -> TokenStream {
    expand_derive(input, schema_inference::derive_infer_schema)
}

#[proc_macro_derive(InferTableFromSchema, attributes(infer_table_from_schema_options))]
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub fn derive_infer_table_from_schema(input: TokenStream) -> TokenStream {
    expand_derive(input, schema_inference::derive_infer_table_from_schema)
}

#[proc_macro_derive(EmbedMigrations, attributes(embed_migrations_options))]
pub fn derive_embed_migrations(input: TokenStream) -> TokenStream {
    expand_derive(input, embed_migrations::derive_embed_migrations)
}

fn expand_derive(input: TokenStream, f: fn(syn::MacroInput) -> quote::Tokens) -> TokenStream {
    let item = parse_macro_input(&input.to_string()).unwrap();
    f(item).to_string().parse().unwrap()
}
