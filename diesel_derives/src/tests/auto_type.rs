use super::AttributeMacro;

use super::expand_with;

#[test]
pub(crate) fn auto_type_1() {
    let input = quote::quote! {
        fn foo() -> _ {
            users::table.select(users::id)
        }
    };
    let attr = Default::default();
    expand_with(
        &crate::auto_type_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::dsl::auto_type)),
        "auto_type_1",
    );
}
