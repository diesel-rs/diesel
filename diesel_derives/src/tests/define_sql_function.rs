use super::FunctionMacro;

use super::expand_with;

#[test]
pub(crate) fn define_sql_function_1() {
    let input = quote::quote! {
        fn lower(input: Text) -> Text;
    };

    let name = if cfg!(feature = "sqlite") {
        "define_sql_function_1 (sqlite)"
    } else {
        "define_sql_function_1"
    };
    expand_with(
        &crate::define_sql_function_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(define_sql_function)),
        name,
    );
}
