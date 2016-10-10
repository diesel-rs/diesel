#![feature(rustc_macro, rustc_macro_lib)]
#![deny(warnings)]

macro_rules! t {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("{}", e),
        }
    };
}

#[cfg(any(feature = "postgres", feature = "sqlite"))]
extern crate diesel_codegen_shared;
extern crate diesel;
#[macro_use]
extern crate quote;
extern crate rustc_macro;
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

use rustc_macro::TokenStream;
use syn::parse_macro_input;

use self::util::{list_value_of_attr_with_name, strip_attributes, strip_field_attributes};

const KNOWN_CUSTOM_DERIVES: &'static [&'static str] = &[
    "AsChangeset",
    "Associations",
    "Identifiable",
    "Insertable",
    "Queryable",
];

const KNOWN_CUSTOM_ATTRIBUTES: &'static [&'static str] = &[
    "belongs_to",
    "changeset_options",
    "has_many",
    "table_name",
];

const KNOWN_FIELD_ATTRIBUTES: &'static [&'static str] = &[
    "column_name",
];

#[rustc_macro_derive(Queryable)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_derive(input, queryable::derive_queryable)
}

#[rustc_macro_derive(Identifiable)]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    expand_derive(input, identifiable::derive_identifiable)
}

#[rustc_macro_derive(Insertable)]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_derive(input, insertable::derive_insertable)
}

#[rustc_macro_derive(AsChangeset)]
pub fn derive_as_changeset(input: TokenStream) -> TokenStream {
    expand_derive(input, as_changeset::derive_as_changeset)
}

#[rustc_macro_derive(Associations)]
pub fn derive_associations(input: TokenStream) -> TokenStream {
    expand_derive(input, associations::derive_associations)
}

#[rustc_macro_derive(InferSchema)]
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub fn derive_infer_schema(input: TokenStream) -> TokenStream {
    let item = parse_macro_input(&input.to_string()).unwrap();
    schema_inference::derive_infer_schema(item)
        .to_string().parse().unwrap()
}

#[rustc_macro_derive(InferTableFromSchema)]
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub fn derive_infer_table_from_schema(input: TokenStream) -> TokenStream {
    let item = parse_macro_input(&input.to_string()).unwrap();
    schema_inference::derive_infer_table_from_schema(item)
        .to_string().parse().unwrap()
}

#[rustc_macro_derive(EmbedMigrations)]
pub fn derive_embed_migratoins(input: TokenStream) -> TokenStream {
    let item = parse_macro_input(&input.to_string()).unwrap();
    embed_migrations::derive_embed_migrations(item)
        .to_string().parse().unwrap()
}

fn expand_derive(input: TokenStream, f: fn(syn::MacroInput) -> quote::Tokens) -> TokenStream {
    let mut item = parse_macro_input(&input.to_string()).unwrap();
    let output = f(item.clone());

    let finished_deriving_diesel_traits = {
        let remaining_derives = list_value_of_attr_with_name(&item.attrs, "derive");
        !remaining_derives
            .unwrap_or(Vec::new())
            .iter()
            .any(|trait_name| KNOWN_CUSTOM_DERIVES.contains(&trait_name.as_ref()))
    };

    if finished_deriving_diesel_traits {
        item.attrs = strip_attributes(item.attrs, KNOWN_CUSTOM_ATTRIBUTES);
        strip_field_attributes(&mut item, KNOWN_FIELD_ATTRIBUTES);
    }

    quote!(#item #output).to_string().parse().unwrap()
}
