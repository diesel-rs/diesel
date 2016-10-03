use syn;
use quote;

use diesel_codegen_shared::load_table_names;

use util::str_value_of_meta_item;

pub fn derive_infer_schema(input: syn::MacroInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!("This is a bug. Please open a Github issue \
               with your invocation of `infer_schema!");
    }

    let options = input.attrs.iter().find(|a| a.name() == "options").map(|a| &a.value);
    let options = match options {
        Some(&syn::MetaItem::List(_, ref options)) => options,
        _ => bug(),
    };
    let database_url = options.iter().find(|a| a.name() == "database_url")
        .map(|a| str_value_of_meta_item(a, "database_url"))
        .unwrap_or_else(|| bug());

    let table_names = load_table_names(&database_url).unwrap();
    let schema_inferences = table_names.into_iter().map(|table_name| {
        quote!(infer_table_from_schema!(#database_url, #table_name);)
    }).collect::<Vec<_>>();

    quote!(#(schema_inferences)*)
}
