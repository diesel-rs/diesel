use super::derive;

use super::expand_with;

#[test]
pub(crate) fn queryable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_queryable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Queryable)])),
        "queryable_1",
    );
}

#[test]
pub(crate) fn queryable_deserialize_as_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(deserialize_as = String)]
            name: MyString
        }
    };

    expand_with(
        &crate::derive_queryable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Queryable)])),
        "queryable_deserialize_as_1",
    );
}
