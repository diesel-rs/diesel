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

#[test]
pub(crate) fn as_changeset_treat_none_as_null_1() {
    let input = quote::quote! {
        #[diesel(treat_none_as_null = true)]
        struct User {
            id: i32,
            name: Option<String>
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_treat_none_as_null_1",
    );
}
