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

pub fn infer_schema_for_schema_name(database_url: &str, schema_name: Option<&str>)
    -> Result<Box<Iterator<Item = (String, Result<quote::Tokens, Box<Error>>)>>, Box<Error>>
{
    let table_names = try!(load_table_names(&database_url, schema_name));
    let schema_name: Option<String> = schema_name.map(String::from);
    let database_url: String = database_url.into();
    Ok(Box::new(table_names.into_iter().map(move |table_name| {
        let mod_ident = syn::Ident::new(format!("infer_{}", table_name));
        let table_name = match schema_name {
            Some(ref name) => format!("{}.{}", name, table_name),
            None => table_name,
        };
        let table = match derive_infer_table_from_schema(&database_url, &table_name) {
            Ok(table) => table,
            Err(e) => {
                return (table_name, Err(e));
            },
        };
        (table_name, Ok(quote! {
            mod #mod_ident {
                #table
            }
            pub use self::#mod_ident::*;
        }))
    })))
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
