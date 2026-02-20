use super::AttributeMacro;

use super::expand_with;

#[test]
pub(crate) fn declare_sql_function_1() {
    let input = quote::quote! {
        extern "SQL" {
            fn lower(input: Text) -> Text;
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "declare_sql_function_1 (sqlite)"
    } else {
        "declare_sql_function_1"
    };
    let attr = Default::default();
    expand_with(
        &crate::declare_sql_function_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::declare_sql_function)),
        name,
    );
}

#[test]
pub(crate) fn declare_sql_function_aggregate_1() {
    let input = quote::quote! {
        extern "SQL" {
            #[aggregate]
            fn my_sum(input: Integer) -> BigInt;
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "declare_sql_function_aggregate_1 (sqlite)"
    } else {
        "declare_sql_function_aggregate_1"
    };
    let attr = Default::default();
    expand_with(
        &crate::declare_sql_function_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::declare_sql_function)),
        name,
    );
}

#[test]
pub(crate) fn declare_sql_function_sql_name_1() {
    let input = quote::quote! {
        extern "SQL" {
            #[sql_name = "MY_LOWER"]
            fn lower(input: Text) -> Text;
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "declare_sql_function_sql_name_1 (sqlite)"
    } else {
        "declare_sql_function_sql_name_1"
    };
    let attr = Default::default();
    expand_with(
        &crate::declare_sql_function_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::declare_sql_function)),
        name,
    );
}

#[test]
pub(crate) fn declare_sql_function_window_1() {
    let input = quote::quote! {
        extern "SQL" {
            #[window]
            fn row_number() -> BigInt;
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "declare_sql_function_window_1 (sqlite)"
    } else {
        "declare_sql_function_window_1"
    };
    let attr = Default::default();
    expand_with(
        &crate::declare_sql_function_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::declare_sql_function)),
        name,
    );
}

#[test]
pub(crate) fn declare_sql_function_variadic_1() {
    let input = quote::quote! {
        extern "SQL" {
            #[variadic(1)]
            fn json_array<V: SqlType + SingleValue>(value: V) -> Json;
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "declare_sql_function_variadic_1 (sqlite)"
    } else {
        "declare_sql_function_variadic_1"
    };
    let attr = Default::default();
    expand_with(
        &crate::declare_sql_function_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::declare_sql_function)),
        name,
    );
}
