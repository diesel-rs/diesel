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
    let table_name = syn::Ident::new(&*table.name);

    let mut tokens = Vec::with_capacity(data.len());

    for a in data {
        tokens.push(column_def_tokens(table, &a, &connection)?);
    }
    let default_schema = default_schema(&connection);
    if table.schema != default_schema {
        if let Some(ref schema) = table.schema {
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
    table: &TableData,
    column: &ColumnInformation,
    connection: &InferConnection,
) -> Result<quote::Tokens, Box<Error>> {
    let column_name = syn::Ident::new(&*column.column_name);
    let column_type = match determine_column_type(column, connection) {
        Ok(t) => t,
        Err(e) => return Err(format!(
            "Error determining type of {}.{}: {}",
            table,
            column.column_name,
            e,
        ).into()),
    };
    let tpe = syn::Path {
        global: false,
        segments: vec![syn::PathSegment::from(column_type.rust_name)],
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

fn default_schema(conn: &InferConnection) -> Option<String> {
    #[cfg(feature="mysql")]
    use information_schema::UsesInformationSchema;
    #[cfg(feature="mysql")]
    use diesel::mysql::Mysql;

    match *conn {
        #[cfg(feature="sqlite")]
        InferConnection::Sqlite(_) => None,
        #[cfg(feature="postgres")]
        InferConnection::Pg(_) => Some("public".into()),
        #[cfg(feature="mysql")]
        InferConnection::Mysql(ref c) => Mysql::default_schema(c).ok(),
    }
}
