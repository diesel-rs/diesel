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
