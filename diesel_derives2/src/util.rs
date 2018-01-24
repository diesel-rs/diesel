use syn::*;
use quote::Tokens;

pub fn wrap_in_dummy_const(const_name: Ident, item: Tokens) -> Tokens {
    quote! {
        const #const_name: () = {
            mod diesel {
                __diesel_use_everything!();
            }
            #item
        };
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
