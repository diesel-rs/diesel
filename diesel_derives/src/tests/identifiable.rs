use super::derive;

use super::expand_with;

#[test]
pub(crate) fn identifiable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_identifiable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Identifiable)])),
        "identifiable_1",
    );
}
