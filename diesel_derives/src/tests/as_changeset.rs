use super::derive;

use super::expand_with;

#[test]
pub(crate) fn as_changeset_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_1",
    );
}
