use super::derive;

use super::expand_with;

#[test]
pub(crate) fn selectable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_selectable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Selectable)])),
        "selectable_1",
    );
}

#[test]
pub(crate) fn selectable_check_for_backend_1() {
    let input = quote::quote! {
        #[diesel(check_for_backend(diesel::pg::Pg, diesel::mysql::Mysql))]
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_selectable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Selectable)])),
        "selectable_check_for_backend_1",
    );
}

#[test]
pub(crate) fn selectable_column_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(column_name = username)]
            name: String
        }
    };

    expand_with(
        &crate::derive_selectable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Selectable)])),
        "selectable_column_name_1",
    );
}

#[test]
pub(crate) fn selectable_embed_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
            #[diesel(embed)]
            address: Address
        }
    };

    expand_with(
        &crate::derive_selectable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Selectable)])),
        "selectable_embed_1",
    );
}

#[test]
pub(crate) fn selectable_select_expression_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(select_expression = users::columns::name.concat(" test"))]
            #[diesel(select_expression_type = diesel::dsl::Concat<users::columns::name, &str>)]
            name: String
        }
    };

    expand_with(
        &crate::derive_selectable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Selectable)])),
        "selectable_select_expression_1",
    );
}
