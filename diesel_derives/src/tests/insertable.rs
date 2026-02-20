use super::derive;

use super::expand_with;

#[test]
pub(crate) fn insertable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_1",
    );
}

#[test]
pub(crate) fn insertable_table_name_1() {
    let input = quote::quote! {
        #[diesel(table_name = crate::schema::admin_users)]
        struct User {
            id: i32,
            name: String,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_table_name_1",
    );
}

#[test]
pub(crate) fn insertable_treat_none_as_default_value_1() {
    let input = quote::quote! {
        #[diesel(treat_none_as_default_value = false)]
        struct User {
            id: i32,
            name: Option<String>,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_treat_none_as_default_value_1",
    );
}

#[test]
pub(crate) fn insertable_column_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(column_name = username)]
            name: String,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_column_name_1",
    );
}

#[test]
pub(crate) fn insertable_embed_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
            #[diesel(embed)]
            address: Address,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_embed_1",
    );
}

#[test]
pub(crate) fn insertable_serialize_as_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(serialize_as = String)]
            name: i32,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_serialize_as_1",
    );
}

#[test]
pub(crate) fn insertable_treat_none_as_default_value_field_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(treat_none_as_default_value = true)]
            name: Option<String>,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_treat_none_as_default_value_field_1",
    );
}

#[test]
pub(crate) fn insertable_skip_insertion_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(skip_insertion)]
            name: String,
        }
    };

    expand_with(
        &crate::derive_insertable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Insertable)])),
        "insertable_skip_insertion_1",
    );
}
