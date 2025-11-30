use super::derive;

use super::expand_with;

#[test]
pub(crate) fn sql_type_1() {
    let input = if cfg!(feature = "postgres") {
        quote::quote! {#[diesel(postgres_type(oid = 42, array_oid = 142))]}
    } else if cfg!(feature = "sqlite") {
        quote::quote! {#[diesel(sqlite_type(name = "Integer"))]}
    } else if cfg!(feature = "mysql") {
        quote::quote! {#[diesel(mysql_type(name = "Long"))]}
    } else {
        unreachable!("At least one feature must be enabled");
    };

    let input = quote::quote! {
        #input
        struct Integer;
    };

    let name = if cfg!(feature = "postgres") {
        "sql_type_1 (postgres)"
    } else if cfg!(feature = "sqlite") {
        "sql_type_1 (sqlite)"
    } else if cfg!(feature = "mysql") {
        "sql_type_1 (mysql)"
    } else {
        unreachable!("At least one feature must be enabled")
    };

    expand_with(
        &crate::derive_sql_type_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(SqlType)])),
        name,
    );
}
