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
pub(crate) fn declare_sql_function_with_named_parameters() {
    let input = quote::quote! {
        extern "SQL" {
            #[named_parameters = true]
            fn lower(input: Text) -> Text;
        }
    };
    let name = if cfg!(feature = "sqlite") {
        "declare_sql_function_with_named_parameters (sqlite)"
    } else {
        "declare_sql_function_with_named_parameters"
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
pub(crate) fn declare_sql_function_query_source() {
    let input = quote::quote! {
        extern "SQL" {
            #[query_source(series, base_type)]
            fn generate_series(start: Integer, stop: Integer) -> series::SqlType;
        }
    };
    let attr = quote::quote! { generate_return_type_helpers = true };
    expand_with(
        &crate::declare_sql_function_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::declare_sql_function)),
        if cfg!(feature = "sqlite") {
            "declare_sql_function_query_source (sqlite)"
        } else {
            "declare_sql_function_query_source"
        },
    );
}

#[test]
fn query_source_generates_sqlite_registration() {
    let output = crate::declare_sql_function_inner(
        Default::default(),
        quote::quote! {
            extern "SQL" {
                #[query_source(grantees)]
                fn grantees(role: Text) -> grantees::SqlType;
            }
        },
    )
    .to_string();
    assert!(output.contains("register_impl"));
    assert!(output.contains("register_table_function"));
}

#[test]
fn query_source_omits_dotted_sqlite_registration_names() {
    let output = crate::declare_sql_function_inner(
        Default::default(),
        quote::quote! {
            extern "SQL" {
                #[query_source(grantees)]
                #[sql_name = "main.grantees"]
                fn grantees(role: Text) -> grantees::SqlType;
            }
        },
    )
    .to_string();
    assert!(!output.contains("register_impl"));
    assert!(!output.contains("compile_error"));
}

#[test]
fn query_source_preserves_external_crate_relations() {
    let input = quote::quote! {
        extern "SQL" {
            #[query_source(other_crate::views::grantees)]
            fn grantees(role: Text) -> other_crate::views::grantees::SqlType;
        }
    };
    let output = crate::declare_sql_function_inner(Default::default(), input).to_string();
    assert!(!output.contains("super :: other_crate"));
}
