use syn;
use quote;

use diesel_codegen_shared::*;

use util::str_value_of_meta_item;

pub fn derive_infer_schema(input: syn::MacroInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!("This is a bug. Please open a Github issue \
               with your invocation of `infer_schema!");
    }

    let options = get_options_from_input(&input.attrs, bug);
    let database_url = get_option(&options, "database_url", bug);

    let table_names = load_table_names(&database_url).unwrap();
    let schema_inferences = table_names.into_iter().map(|table_name| {
        quote!(infer_table_from_schema!(#database_url, #table_name);)
    });

    quote!(#(schema_inferences)*)
}

pub fn derive_infer_table_from_schema(input: syn::MacroInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!("This is a bug. Please open a Github issue \
               with your invocation of `infer_table_from_schema!");
    }

    let options = get_options_from_input(&input.attrs, bug);
    let database_url = get_option(options, "database_url", bug);
    let table_name = get_option(options, "table_name", bug);

    let connection = establish_connection(database_url).unwrap();
    let data = get_table_data(&connection, table_name).unwrap();
    let primary_keys = get_primary_keys(&connection, table_name).unwrap()
        .into_iter().map(syn::Ident::new);
    let table_name = syn::Ident::new(table_name);

    let tokens = data.iter().map(|a| column_def_tokens(a, &connection));

    quote!(table! {
        #table_name (#(primary_keys),*) {
            #(tokens,)*
        }
    })
}

fn column_def_tokens(
    column: &ColumnInformation,
    connection: &InferConnection,
) -> quote::Tokens {
    let column_name = syn::Ident::new(&*column.column_name);
    let column_type = determine_column_type(column, connection).unwrap();
    let path_segments = column_type.path
        .into_iter()
        .map(syn::Ident::new)
        .map(syn::PathSegment::ident)
        .collect();
    let tpe = syn::Path { global: true, segments: path_segments };
    let mut tpe = quote!(#tpe);

    if column_type.is_array {
        tpe = quote!(Array<#tpe>);
    }
    if column_type.is_nullable {
        tpe = quote!(Nullable<#tpe>);
    }
    quote!(#column_name -> #tpe)
}

fn get_options_from_input(attrs: &[syn::Attribute], on_bug: fn() -> !) -> &[syn::MetaItem] {
    let options = attrs.iter().find(|a| a.name() == "options").map(|a| &a.value);
    match options {
        Some(&syn::MetaItem::List(_, ref options)) => options,
        _ => on_bug(),
    }
}

fn get_option<'a>(
    options: &'a [syn::MetaItem],
    option_name: &str,
    on_bug: fn() -> !,
) -> &'a str {
    options.iter().find(|a| a.name() == option_name)
        .map(|a| str_value_of_meta_item(a, option_name))
        .unwrap_or_else(|| on_bug())
}
