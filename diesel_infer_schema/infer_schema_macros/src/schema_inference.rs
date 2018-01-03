use syn;
use quote;

use database_url::extract_database_url;
use infer_schema_internals::*;

use util::{get_option, get_optional_option, get_options_from_input};

pub fn derive_infer_schema(input: syn::DeriveInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!(
            "This is a bug. Please open a Github issue \
             with your invocation of `infer_schema`!"
        );
    }

    let options =
        get_options_from_input("infer_schema_options", &input.attrs, bug).unwrap_or_else(|| bug());
    let database_url = extract_database_url(get_option(&options, "database_url", bug)).unwrap();
    let schema_name = get_optional_option(&options, "schema_name");
    let schema_name = schema_name.as_ref().map(|s| &**s);

    let table_names = load_table_names(&database_url, schema_name).expect(&error_message(
        "table names",
        &database_url,
        schema_name,
    ));
    let foreign_keys = load_foreign_key_constraints(&database_url, schema_name)
        .expect(&error_message("foreign keys", &database_url, schema_name));
    let foreign_keys =
        remove_unsafe_foreign_keys_for_codegen(&database_url, &foreign_keys, &table_names);

    let tables = table_names.iter().map(|table| {
        let mod_ident = syn::Ident::new(format!("infer_{}", table.name));
        let table_name = table.to_string();
        quote! {
            mod #mod_ident {
                infer_table_from_schema!(#database_url, #table_name);
            }
            pub use self::#mod_ident::*;
        }
    });
    let joinables = foreign_keys.into_iter().map(|fk| {
        let child_table = syn::Ident::new(fk.child_table.name);
        let parent_table = syn::Ident::new(fk.parent_table.name);
        let foreign_key = syn::Ident::new(fk.foreign_key);
        quote!(joinable!(#child_table -> #parent_table (#foreign_key));)
    });

    let table_idents = table_names.iter().map(|t| syn::Ident::from(&*t.name));
    let multi_table_joins = quote!(allow_tables_to_appear_in_same_query!(#(#table_idents,)*););

    let tokens = quote!(#(#tables)* #(#joinables)* #multi_table_joins);
    if let Some(schema_name) = schema_name {
        let schema_ident = syn::Ident::new(schema_name);
        quote!(pub mod #schema_ident { #tokens })
    } else {
        tokens
    }
}

pub fn derive_infer_table_from_schema(input: syn::DeriveInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!(
            "This is a bug. Please open a Github issue \
             with your invocation of `infer_table_from_schema`!"
        );
    }

    let options = get_options_from_input("infer_table_from_schema_options", &input.attrs, bug)
        .unwrap_or_else(|| bug());
    let database_url = extract_database_url(get_option(&options, "database_url", bug)).unwrap();
    let table_name = get_option(&options, "table_name", bug);
    let table_data = load_table_data(&database_url, table_name.parse().unwrap())
        .expect(&error_message(table_name, &database_url, None));

    table_data_to_tokens(table_data)
}

fn error_message(attempted_to_load: &str, database_url: &str, schema_name: Option<&str>) -> String {
    let mut message = format!(
        "Could not load {} from database `{}`",
        attempted_to_load, database_url
    );
    if let Some(name) = schema_name {
        message += &format!(" with schema `{}`", name);
    }
    message
}

fn table_data_to_tokens(table_data: TableData) -> quote::Tokens {
    let table_docs = to_doc_comment_tokens(&table_data.docs);
    let table_name = table_name_to_tokens(table_data.name);
    let primary_key = table_data.primary_key.into_iter().map(syn::Ident::new);
    let column_definitions = table_data
        .column_data
        .into_iter()
        .map(column_data_to_tokens);
    quote! {
        table! {
            #(#table_docs)*
            #table_name (#(#primary_key),*) {
                #(#column_definitions),*,
            }
        }
    }
}

fn table_name_to_tokens(table_name: TableName) -> quote::Tokens {
    let name = syn::Ident::new(table_name.name);
    if let Some(schema) = table_name.schema {
        let schema = syn::Ident::new(schema);
        quote!(#schema.#name)
    } else {
        quote!(#name)
    }
}

fn column_data_to_tokens(column_data: ColumnDefinition) -> quote::Tokens {
    let docs = to_doc_comment_tokens(&column_data.docs);
    let ty = column_ty_to_tokens(column_data.ty);
    if let Some(rust_name) = column_data.rust_name {
        let rust_name = syn::Ident::new(rust_name);
        let sql_name = column_data.sql_name;

        quote!(
            #(#docs)*
            #[sql_name = #sql_name]
            #rust_name -> #ty
        )
    } else {
        let name = syn::Ident::new(column_data.sql_name);

        quote!(
            #(#docs)*
            #name -> #ty
        )
    }
}

fn column_ty_to_tokens(column_ty: ColumnType) -> quote::Tokens {
    let name = syn::Ident::new(column_ty.rust_name);
    let mut tokens = quote!(#name);
    if column_ty.is_array {
        tokens = quote!(Array<#tokens>);
    }
    if column_ty.is_nullable {
        tokens = quote!(Nullable<#tokens>);
    }
    tokens
}

fn to_doc_comment_tokens(docs: &str) -> Vec<syn::Token> {
    docs.lines()
        .map(|l| format!("///{}{}", if l.is_empty() { "" } else { " " }, l))
        .map(syn::Token::DocComment)
        .collect()
}
