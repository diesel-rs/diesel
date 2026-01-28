use super::FunctionMacro;

use super::expand_with;

#[test]
pub(crate) fn diesel_for_each_tuple_1() {
    let input = quote::quote! {
        tuple_impls
    };

    expand_with(
        &crate::__diesel_for_each_tuple_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(__diesel_for_each_tuple)),
        "diesel_for_each_tuple_1",
    );
}
