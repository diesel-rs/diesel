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
        "declare_sql_function_query_source",
    );
}

#[test]
pub(crate) fn declare_sql_function_generic_query_source() {
    let input = quote::quote! {
        extern "SQL" {
            #[query_source(R, base_type)]
            fn generate_series<R: diesel::query_source::SingleColumnRelation<ColumnSqlType = ST>, ST: SqlType + TypedExpressionType>(
                start: ST,
                stop: ST,
            ) -> R::SqlType;

            #[query_source(R, base_type)]
            fn series_relation_last<ST: SqlType + TypedExpressionType, R: diesel::query_source::SingleColumnRelation<ColumnSqlType = ST>>(
                start: ST,
                stop: ST,
            ) -> R::SqlType;
        }
    };
    let attr = quote::quote! { generate_return_type_helpers = true };
    expand_with(
        &crate::declare_sql_function_inner as &dyn Fn(_, _) -> _,
        (attr, input),
        AttributeMacro(syn::parse_quote!(diesel::declare_sql_function)),
        "declare_sql_function_generic_query_source",
    );
}
