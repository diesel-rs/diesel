use std::error::Error;

use quote;
use syn;

use table_data::TableData;
use data_structures::ColumnInformation;
use inference::{establish_connection, get_table_data, determine_column_type, get_primary_keys,
                InferConnection};

pub fn expand_infer_table_from_schema(database_url: &str, table: &TableData)
    -> Result<quote::Tokens, Box<Error>>
{
    let connection = establish_connection(database_url)?;
    let data = get_table_data(&connection, table)?;
    let primary_keys = get_primary_keys(&connection, table)?
        .into_iter()
        .map(syn::Ident::new);
    let table_name = syn::Ident::new(&table.name()[..]);

    let mut tokens = Vec::with_capacity(data.len());

    for a in data {
        tokens.push(column_def_tokens(&a, &connection)?);
    }
    if let Some(ref schema) = *table.schema() {
        if cfg!(not(feature = "postgres")) || schema != "public" {
            let schema_name = syn::Ident::new(&schema[..]);
            return Ok(quote!(table! {
                #schema_name.#table_name (#(#primary_keys),*) {
                    #(#tokens),*,
                }
            }));
        }
    }
    Ok(quote!(table! {
        #table_name (#(#primary_keys),*) {
            #(#tokens),*,
        }
    }))
}

pub fn handle_schema<I>(tables: I, schema_name: Option<&str>) -> quote::Tokens
    where I: Iterator<Item = quote::Tokens>
{
    match schema_name {
        Some(name) => {
            let schema_ident = syn::Ident::new(name);
            quote! { pub mod #schema_ident { #(#tables)* } }
        }
        None => quote!(#(#tables)*),
    }
}

fn column_def_tokens(
    column: &ColumnInformation,
    connection: &InferConnection,
) -> Result<quote::Tokens, Box<Error>> {
    let column_name = syn::Ident::new(&*column.column_name);
    let column_type = try!(determine_column_type(column, connection));
    let tpe = if column_type.path[0] == "diesel" && column_type.path[1] == "types" {
        let path_segments = column_type.path
            .into_iter()
            .skip(2)
            .map(syn::PathSegment::from)
            .collect();
        syn::Path { global: false, segments: path_segments }
    } else {
        let path_segments = column_type.path
            .into_iter()
            .map(syn::PathSegment::from)
            .collect();
        syn::Path { global: true, segments: path_segments }
    };
    let mut tpe = quote!(#tpe);

    if column_type.is_array {
        tpe = quote!(Array<#tpe>);
    }
    if column_type.is_nullable {
        tpe = quote!(Nullable<#tpe>);
    }
    Ok(quote!(#column_name -> #tpe))
}
