use proc_macro2::Span;
use quote::Tokens;
use syn::*;

pub use diagnostic_shim::*;

pub fn wrap_in_dummy_mod(const_name: Ident, item: Tokens) -> Tokens {
    let call_site = root_span(Span::call_site());
    let use_everything = quote_spanned!(call_site=> __diesel_use_everything!());
    quote! {
        #[allow(non_snake_case)]
        mod #const_name {
            // https://github.com/rust-lang/rust/issues/47314
            extern crate std;

            mod diesel {
                #use_everything;
            }
            #item
        }
    }
}

pub fn inner_of_option_ty(ty: &Type) -> &Type {
    option_ty_arg(ty).unwrap_or(ty)
}

pub fn is_option_ty(ty: &Type) -> bool {
    option_ty_arg(ty).is_some()
}

fn option_ty_arg(ty: &Type) -> Option<&Type> {
    use syn::PathArguments::AngleBracketed;

    match *ty {
        Type::Path(ref ty) => {
            let last_segment = ty.path.segments.iter().last().unwrap();
            match last_segment.arguments {
                AngleBracketed(ref args) if last_segment.ident == "Option" => {
                    match args.args.iter().last() {
                        Some(&GenericArgument::Type(ref ty)) => Some(ty),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(not(feature = "nightly"))]
fn root_span(span: Span) -> Span {
    span
}

#[cfg(feature = "nightly")]
/// There's an issue with the resolution of `__diesel_use_everything` if the
/// derive itself was generated from within a macro. This is a shitty workaround
/// until we figure out the expected behavior.
fn root_span(span: Span) -> Span {
    span.unstable().source().into()
}
