#[test]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub(crate) fn sql_function_1() {
    let input = quote::quote! {
        fn lower(input: Text) -> Text;
    };

    let name = if cfg!(feature = "sqlite") {
        "sql_function_1 (sqlite)"
    } else {
        "sql_function_1"
    };
    super::expand_with(
        &crate::sql_function_proc_inner as &dyn Fn(_) -> _,
        input,
        super::FunctionMacro(syn::parse_quote!(sql_function)),
        name,
    );
}
