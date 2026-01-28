use super::derive;

use super::expand_with;

#[test]
pub(crate) fn from_sql_row_1() {
    let input = quote::quote! {
        enum Foo {
            Bar,
            Baz
        }
    };

    expand_with(
        &crate::derive_from_sql_row_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(FromSqlRow)])),
        "from_sql_row_1",
    );
}
