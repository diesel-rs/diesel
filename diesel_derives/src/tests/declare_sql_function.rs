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
