use super::FunctionMacro;
use super::expand_with;

#[test]
pub(crate) fn composite_type_1() {
    let input = quote::quote! {
        test_type {
            a -> Integer,
            b -> Text,
        }
    };

    expand_with(
        &crate::composite_type_proc_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(composite_type)),
        "composite_type_1",
    );
}

#[test]
pub(crate) fn composite_type_reserved_rust_keyword() {
    let input = quote::quote! {
        r#type.r#u32 {
            a -> Integer,
            b -> Text,
        }
    };

    expand_with(
        &crate::composite_type_proc_inner as &dyn Fn(_) -> _,
        input,
        FunctionMacro(syn::parse_quote!(composite_type)),
        "composite_type_reserved_rust_keyword",
    );
}
