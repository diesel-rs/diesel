use super::expand_with;
use super::AttributeMacro;

#[test]
pub(crate) fn diesel_public_if_1() {
    let input = quote::quote! {
        pub(crate) mod example;
    };
    let attr = quote::quote! {
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    };

    expand_with(
        &crate::__diesel_public_if_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel_derives::__diesel_public_if)),
        "diesel_public_if_1",
    );
}
