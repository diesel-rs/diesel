use super::FunctionMacro;
use super::expand_with;

#[test]
pub(crate) fn table_1() {
    let input = quote::quote! {
        users {
            id -> Integer,
            name -> Text,
        }
    };
    let name = if cfg!(feature = "postgres") {
        "table_1 (postgres)"
    } else {
        "table_1"
    };

    expand_with(
        &crate::table_proc_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(table)),
        name,
    );
}
