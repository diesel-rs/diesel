use quote;
use syn;

use std::error::Error;
use data_structures::ColumnInformation;
use inference::{establish_connection, get_table_data, determine_column_type,
                get_primary_keys, load_table_names, InferConnection};


pub fn derive_infer_table_from_schema(database_url: &str, table_name: &str)
    -> Result<quote::Tokens, Box<Error>>
{

    let connection = try!(establish_connection(database_url));
    let data = try!(get_table_data(&connection, table_name));
    let primary_keys = try!(get_primary_keys(&connection, table_name))
        .into_iter().map(syn::Ident::new);
    let table_name = syn::Ident::new(table_name);

    let mut tokens = Vec::with_capacity(data.len());
    for a in data {
        tokens.push(try!(column_def_tokens(&a, &connection)));
    }

    Ok(quote!(table! {
        #table_name (#(#primary_keys),*) {
            #(#tokens),*,
        }
    }))
}

pub fn infer_schema_for_schema_name<F>(database_url: &str, schema_name: Option<&str>, error_handler: F)
    -> Result<quote::Tokens, Box<Error>>
    where for<'a> F: Fn(&'a str, Box<Error>),
{
    let table_names = try!(load_table_names(&database_url, schema_name));
    let schema_inferences = table_names.into_iter().filter_map(|table_name| {
        let mod_ident = syn::Ident::new(format!("infer_{}", table_name));
        let table_name = match schema_name {
            Some(name) => format!("{}.{}", name, table_name),
            None => table_name,
        };
        let table = match derive_infer_table_from_schema(database_url, &table_name){
            Ok(table) => table,
            Err(e) => {
                error_handler(&table_name, e);
                return None;
            },
        };
        Some(quote! {
            mod #mod_ident {
                #table
            }
            pub use self::#mod_ident::*;
        })
    });

    match schema_name {
        Some(name) => {
            let schema_ident = syn::Ident::new(name);
            Ok(quote! { pub mod #schema_ident { #(#schema_inferences)* } })
        }
        None => Ok(quote!(#(#schema_inferences)*)),
    }
}

fn column_def_tokens(
    column: &ColumnInformation,
    connection: &InferConnection,
) -> Result<quote::Tokens, Box<Error>> {
    let column_name = syn::Ident::new(&*column.column_name);
    let column_type = try!(determine_column_type(column, connection));
    let path_segments = column_type.path
        .into_iter()
        .map(syn::PathSegment::from)
        .collect();
    let tpe = syn::Path { global: true, segments: path_segments };
    let mut tpe = quote!(#tpe);

    if column_type.is_array {
        tpe = quote!(Array<#tpe>);
    }
    if column_type.is_nullable {
        tpe = quote!(Nullable<#tpe>);
    }
    Ok(quote!(#column_name -> #tpe))
}
