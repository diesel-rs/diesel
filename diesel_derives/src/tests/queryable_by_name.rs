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

#[test]
pub(crate) fn queryable_by_name_2() {
    let input = quote::quote! {
        pub struct UserCount {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            pub value: i16,
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_2",
    );
}
