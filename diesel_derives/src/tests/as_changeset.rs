use super::derive;

use super::expand_with;

#[test]
pub(crate) fn as_changeset_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_1",
    );
}

#[test]
pub(crate) fn as_changeset_treat_none_as_null_1() {
    let input = quote::quote! {
        #[diesel(treat_none_as_null = true)]
        struct User {
            id: i32,
            name: Option<String>
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_treat_none_as_null_1",
    );
}

#[test]
pub(crate) fn as_changeset_table_name_1() {
    let input = quote::quote! {
        #[diesel(table_name = crate::schema::admin_users)]
        struct User {
            id: i32,
            name: String
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_table_name_1",
    );
}

#[test]
pub(crate) fn as_changeset_primary_key_1() {
    let input = quote::quote! {
        #[diesel(primary_key(id, short_code))]
        struct User {
            id: i32,
            short_code: String,
            name: String
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_primary_key_1",
    );
}

#[test]
pub(crate) fn as_changeset_column_name_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(column_name = username)]
            name: String
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_column_name_1",
    );
}

#[test]
pub(crate) fn as_changeset_embed_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
            #[diesel(embed)]
            post: Post
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_embed_1",
    );
}

#[test]
pub(crate) fn as_changeset_change_field_type_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String,
            #[diesel(serialize_as = String)]
            age: i32
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_change_field_type_1",
    );
}

#[test]
pub(crate) fn as_changeset_treat_none_field_as_null_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(treat_none_as_null = true)]
            name: Option<String>
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_treat_none_field_as_null_1",
    );
}

#[test]
pub(crate) fn as_changeset_treat_skip_update_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            #[diesel(skip_update)]
            name: String
        }
    };
    expand_with(
        &crate::derive_as_changeset_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(AsChangeset)])),
        "as_changeset_treat_skip_update_1",
    );
}
