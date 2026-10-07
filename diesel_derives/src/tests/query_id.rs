use super::derive;

use super::expand_with;

#[test]
pub(crate) fn query_id_1() {
    let input = quote::quote! {
        struct Query;
    };

    expand_with(
        &crate::derive_query_id_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryId)])),
        "query_id_1",
    );
}

#[test]
pub(crate) fn query_id_lifetime() {
    let input = quote::quote! {
        struct Query<'a> { f: &'a str }
    };

    expand_with(
        &crate::derive_query_id_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryId)])),
        "query_id_lifetime",
    );
}

#[test]
pub(crate) fn query_id_bounded_lifetimes() {
    let input = quote::quote! {
        struct Query<'a: 'b, 'b> { f: &'a str, g: &'b str }
    };

    expand_with(
        &crate::derive_query_id_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryId)])),
        "query_id_bounded_lifetimes",
    );
}

#[test]
pub(crate) fn query_id_const_before_type() {
    let input = quote::quote! {
        struct Query<'a, const N: usize, T> { f: &'a str, g: T }
    };

    expand_with(
        &crate::derive_query_id_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryId)])),
        "query_id_const_before_type",
    );
}
