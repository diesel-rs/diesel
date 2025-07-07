use super::derive;

use super::expand_with;

#[test]
pub(crate) fn has_query_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "has_query_1 (sqlite)"
    } else if cfg!(feature = "postgres") {
        "has_query_1 (postgres)"
    } else if cfg!(feature = "mysql") {
        "has_query_1 (mysql)"
    } else {
        unimplemented!()
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        name,
    );
}
