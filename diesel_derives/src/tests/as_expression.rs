use super::derive;

use super::expand_with;

#[test]
pub(crate) fn as_expression_1() {
    let input = quote::quote! {
        #[diesel(sql_type = diesel::sql_type::Integer)]
        enum Foo {
            Bar,
            Baz
        }
    };
    expand_with(
        &crate::derive_as_expression_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsExpression)])),
        "as_expression_1",
    );
}
