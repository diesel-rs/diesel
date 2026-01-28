use super::derive;

use super::expand_with;

#[test]
pub(crate) fn diesel_numeric_ops_1() {
    let input = quote::quote! {
        struct NumericColumn;
    };

    expand_with(
        &crate::derive_diesel_numeric_ops_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(DieselNumericOps)])),
        "diesel_numeric_ops_1",
    );
}
