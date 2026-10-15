use super::derive;

use super::expand_with;

#[test]
pub(crate) fn has_query_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "has_query_1 (sqlite)"
    } else if cfg!(feature = "postgres") {
        "has_query_1 (postgres)"
    } else if cfg!(feature = "mysql") {
        "has_query_1 (mysql)"
    } else if cfg!(feature = "mariadb") {
        "has_query_1 (mariadb)"
    } else {
        unimplemented!()
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        name,
    );
}

#[test]
pub(crate) fn has_query_custom_base_query() {
    let input = quote::quote! {
        #[diesel(base_query = users::table.filter(users::id.eq(1_i32)))]
        struct User {
            id: i32,
            name: String
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "has_query_custom_base_query (sqlite)"
    } else if cfg!(feature = "postgres") {
        "has_query_custom_base_query (postgres)"
    } else if cfg!(feature = "mysql") {
        "has_query_custom_base_query (mysql)"
    } else if cfg!(feature = "mariadb") {
        "has_query_custom_base_query (mariadb)"
    } else {
        unimplemented!()
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        name,
    );
}
