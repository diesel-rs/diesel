use super::derive;

use super::expand_with;

#[test]
pub(crate) fn queryable_by_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_1",
    );
}
