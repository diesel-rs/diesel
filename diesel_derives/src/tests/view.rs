use super::expand_with;
use super::FunctionMacro;

#[test]
pub(crate) fn view_1() {
    let input = quote::quote! {
        view {
            id -> Integer,
            name -> Text,
        }
    };
    let name = if cfg!(feature = "postgres") {
        "view_1 (postgres)"
    } else {
        "view_1"
    };

    expand_with(
        &crate::view_proc_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(view)),
        name,
    );
}
