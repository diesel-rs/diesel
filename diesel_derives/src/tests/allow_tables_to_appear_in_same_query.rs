use super::expand_with;
use super::FunctionMacro;
use crate::allow_tables_to_appear_in_same_query::expand;

#[test]
fn simple() {
    let input = quote::quote! {
        users, posts, comments
    };

    expand_with(
        &expand as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(allow_tables_to_appear_in_same_query)),
        "simple",
    );
}

#[test]
fn with_paths() {
    let input = quote::quote! {
        schema::users, schema::posts, comments
    };

    expand_with(
        &expand as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(allow_tables_to_appear_in_same_query)),
        "with_paths",
    );
}
