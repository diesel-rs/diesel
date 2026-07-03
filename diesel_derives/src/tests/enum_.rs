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

    expand_with(
        &crate::derive_enum_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Enum)])),
        "enum_1",
    );
}

#[test]
pub(crate) fn enum_2() {
    let input = quote::quote! {
        #[derive(Debug, diesel::Enum)]
        #[diesel(sql_type = diesel::sql_types::Integer)]
        enum Color {
            Red = 1,
            Green = 2,
            Blue = 3
        }
    };

    expand_with(
        &crate::derive_enum_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Enum)])),
        "enum_2",
    );
}

#[test]
fn rename_all() {
    let input = quote::quote! {
        #[derive(Debug, diesel::Enum)]
        #[diesel(sql_type = diesel::sql_types::Text)]
        #[diesel(rename_all = "snake_case")]
        enum Color {
            RedColor,
            GreenColor,
            BlueColor,
        }
    };

    expand_with(
        &crate::derive_enum_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Enum)])),
        "enum_rename_all",
    );
}

#[test]
fn rename_single() {
    let input = quote::quote! {
        #[derive(Debug, diesel::Enum)]
        #[diesel(sql_type = diesel::sql_types::Text)]
        enum Color {
            #[diesel(rename = "ReD")]
            Red,
            #[diesel(rename = "GreeN")]
            Green,
            #[diesel(rename = "BluE")]
            Blue
        }
    };

    expand_with(
        &crate::derive_enum_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Enum)])),
        "enum_rename_single",
    );
}
