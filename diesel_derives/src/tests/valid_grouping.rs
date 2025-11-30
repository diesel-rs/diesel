use super::derive;

use super::expand_with;

#[test]
pub(crate) fn valid_grouping_1() {
    let input = quote::quote! {
        struct Query;
    };

    expand_with(
        &crate::derive_valid_grouping_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(ValidGrouping)])),
        "valid_grouping_1",
    );
}
