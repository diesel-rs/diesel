pub use diagnostic_shim::{Diagnostic, DiagnosticShim, EmitErrorExt};

use proc_macro2::{TokenStream, Span};
use syn::{Ident, Type, DeriveInput, Data, GenericArgument};
use meta::MetaItem;

pub fn wrap_in_dummy_mod(const_name: Ident, item: TokenStream) -> TokenStream {
    quote! {
        #[allow(non_snake_case, unused_extern_crates, unused_imports)]
        fn #const_name() {
            // https://github.com/rust-lang/rust/issues/47314
            extern crate std;
            use diesel;

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

pub fn ty_for_foreign_derive(item: &DeriveInput, flags: &MetaItem) -> Result<Type, Diagnostic> {
    if flags.has_flag("foreign_derive") {
        match item.data {
            Data::Struct(ref body) => match body.fields.iter().nth(0) {
                Some(field) => Ok(field.ty.clone()),
                None => Err(flags
                    .span()
                    .error("foreign_derive requires at least one field")),
            },
            _ => Err(flags
                .span()
                .error("foreign_derive can only be used with structs")),
        }
    } else {
        let ident = &item.ident;
        let (_, ty_generics, ..) = item.generics.split_for_impl();
        Ok(parse_quote!(#ident #ty_generics))
    }
}

pub fn fix_span(maybe_bad_span: Span, mut fallback: Span) -> Span {
    let bad_span_debug = "#0 bytes(0..0)";

    if format!("{:?}", fallback) == bad_span_debug {
        // On recent rust nightlies, even our fallback span is bad.
        fallback = Span::call_site();
    }

    if format!("{:?}", maybe_bad_span) == bad_span_debug {
        fallback
    } else {
        maybe_bad_span
    }
}
