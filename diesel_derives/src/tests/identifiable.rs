use super::derive;

use super::expand_with;

#[test]
pub(crate) fn identifiable_1() {
    let input = quote::quote! {
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_identifiable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Identifiable)])),
        "identifiable_1",
    );
}

#[test]
pub(crate) fn identifiable_table_name_1() {
    let input = quote::quote! {
        #[diesel(table_name = crate::schema::admin_users)]
        struct User {
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_identifiable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Identifiable)])),
        "identifiable_table_name_1",
    );
}

#[test]
pub(crate) fn identifiable_primary_key_1() {
    let input = quote::quote! {
        #[diesel(primary_key(id, short_code))]
        struct User {
            id: i32,
            short_code: String,
            name: String
        }
    };

    expand_with(
        &crate::derive_identifiable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Identifiable)])),
        "identifiable_primary_key_1",
    );
}

#[test]
pub(crate) fn identifiable_column_name_1() {
    let input = quote::quote! {
        #[diesel(primary_key(user_id))]
        struct User {
            #[diesel(column_name = user_id)]
            id: i32,
            name: String
        }
    };

    expand_with(
        &crate::derive_identifiable_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Identifiable)])),
        "identifiable_column_name_1",
    );
}
