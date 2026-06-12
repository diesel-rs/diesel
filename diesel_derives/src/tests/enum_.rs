use super::derive;
use super::expand_with;

#[test]
pub(crate) fn enum_1() {
    let input = quote::quote! {
        #[derive(Debug, diesel::Enum)]
        #[diesel(sql_type = schema::sql_types::Color)]
        enum Color {
            Red,
            Green,
            Blue
        }
    };

    let name = if cfg!(feature = "postgres") {
        "postgres"
    } else if cfg!(feature = "mysql") {
        "mysql"
    } else {
        // no support for sqlite yet
        return;
    };

    expand_with(
        &crate::derive_enum_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Enum)])),
        &format!("enum1_({name})"),
    );
}
