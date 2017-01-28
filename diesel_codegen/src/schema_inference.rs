use syn;
use quote;

use diesel_codegen_shared::extract_database_url;
use diesel_infer_schema;

use util::{get_options_from_input, get_option, get_optional_option};

pub fn derive_infer_schema(input: syn::MacroInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!("This is a bug. Please open a Github issue \
               with your invocation of `infer_schema!");
    }

    let options = get_options_from_input("infer_schema_options", &input.attrs, bug)
        .unwrap_or_else(|| bug());
    let database_url = extract_database_url(get_option(&options, "database_url", bug)).unwrap();
    let schema_name = get_optional_option(&options, "schema_name");
    let schema_name = schema_name.as_ref().map(|s| &**s);

    let table_names = diesel_infer_schema::load_table_names(&database_url, schema_name)
        .expect(&format!("Could not load table names from database `{}`{}",
            database_url,
            if let Some(name) = schema_name {
                format!(" with schema `{}`", name)
            } else {
                "".into()
            }
        ));
    
    let tables = table_names.iter()
        .map(|table| {
            diesel_infer_schema::infer_schema_for_schema_name(table, &database_url)
              .expect(&format!("Could not load table `{}`", table.to_string()))
        })
        .map(|(_table, tokens)| tokens);

    diesel_infer_schema::handle_schema(tables, schema_name)
}

pub fn derive_infer_table_from_schema(input: syn::MacroInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!("This is a bug. Please open a Github issue \
               with your invocation of `infer_table_from_schema!");
    }

    let options = get_options_from_input("infer_table_from_schema_options", &input.attrs, bug)
        .unwrap_or_else(|| bug());
    let database_url = extract_database_url(get_option(&options, "database_url", bug)).unwrap();
    let table_name = get_option(&options, "table_name", bug);

    diesel_infer_schema::derive_infer_table_from_schema(&database_url, table_name)
        .expect(&format!("Could not infer table {}", table_name))
}

