use super::derive;

use super::expand_with;

#[test]
pub(crate) fn queryable_by_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_1",
    );
}

#[test]
pub(crate) fn queryable_by_name_table_name_1() {
    let input = quote::quote! {
        #[diesel(table_name = crate::schema::users)]
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_table_name_1",
    );
}

#[test]
pub(crate) fn queryable_by_name_check_for_backend_1() {
    let input = quote::quote! {
        #[diesel(check_for_backend(diesel::pg::Pg, diesel::mysql::Mysql))]
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_check_for_backend_1",
    );
}

#[test]
pub(crate) fn queryable_by_name_column_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(column_name = username)]
            name: String
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_column_name_1",
    );
}

#[test]
pub(crate) fn queryable_by_name_sql_type_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_sql_type_1",
    );
}

#[test]
pub(crate) fn queryable_by_name_deserialize_as_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(deserialize_as = String)]
            name: MyString
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_deserialize_as_1",
    );
}

#[test]
pub(crate) fn queryable_by_name_embed_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
            #[diesel(embed)]
            address: Address
        }
    };

    expand_with(
        &crate::derive_queryable_by_name_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(QueryableByName)])),
        "queryable_by_name_embed_1",
    );
}
