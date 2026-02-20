use super::derive;

use super::expand_with;

fn has_query_snapshot_name(base: &str) -> &'static str {
    if cfg!(feature = "sqlite") {
        Box::leak(format!("{base} (sqlite)").into_boxed_str())
    } else if cfg!(feature = "postgres") {
        Box::leak(format!("{base} (postgres)").into_boxed_str())
    } else if cfg!(feature = "mysql") {
        Box::leak(format!("{base} (mysql)").into_boxed_str())
    } else {
        unimplemented!()
    }
}

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
pub(crate) fn has_query_base_query_1() {
    let input = quote::quote! {
        #[diesel(base_query = users::table.order_by(users::id))]
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_base_query_1"),
    );
}

#[test]
pub(crate) fn has_query_base_query_type_1() {
    let input = quote::quote! {
        #[diesel(base_query = users::table.order_by(users::id))]
        #[diesel(base_query_type = diesel::dsl::Order<users::table, users::id>)]
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_base_query_type_1"),
    );
}

#[test]
pub(crate) fn has_query_table_name_1() {
    let input = quote::quote! {
        #[diesel(table_name = crate::schema::admin_users)]
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_table_name_1"),
    );
}

#[test]
pub(crate) fn has_query_check_for_backend_1() {
    let input = quote::quote! {
        #[diesel(check_for_backend(diesel::pg::Pg, diesel::mysql::Mysql))]
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_check_for_backend_1"),
    );
}

#[test]
pub(crate) fn has_query_check_for_backend_disable_1() {
    let input = quote::quote! {
        #[diesel(check_for_backend(disable = true))]
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_check_for_backend_disable_1"),
    );
}

#[test]
pub(crate) fn has_query_column_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(column_name = username)]
            name: String
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_column_name_1"),
    );
}

#[test]
pub(crate) fn has_query_embed_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
            #[diesel(embed)]
            address: Address
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_embed_1"),
    );
}

#[test]
pub(crate) fn has_query_select_expression_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(select_expression = users::columns::name.concat(" test"))]
            #[diesel(select_expression_type = diesel::dsl::Concat<users::columns::name, &str>)]
            name: String
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_select_expression_1"),
    );
}

#[test]
pub(crate) fn has_query_deserialize_as_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(deserialize_as = String)]
            name: MyString
        }
    };
    expand_with(
        &crate::derive_has_query_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(HasQuery)])),
        has_query_snapshot_name("has_query_deserialize_as_1"),
    );
}
