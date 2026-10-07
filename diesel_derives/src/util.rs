use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Result;
use syn::{Data, DeriveInput, GenericArgument, Type, parse_quote};

use crate::model::Model;

pub fn wrap_in_dummy_mod(item: TokenStream) -> TokenStream {
    quote! {
        const _: () = {
            // This import is not actually redundant. When using diesel_derives
            // inside of diesel, `diesel` doesn't exist as an extern crate, and
            // to work around that it contains a private
            // `mod diesel { pub use super::*; }` that this import will then
            // refer to. In all other cases, this imports refers to the extern
            // crate diesel.
            use diesel;

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

fn option_ty_arg(mut ty: &Type) -> Option<&Type> {
    use syn::PathArguments::AngleBracketed;

    // Check the inner equivalent type
    loop {
        match ty {
            Type::Group(group) => ty = &group.elem,
            Type::Paren(paren) => ty = &paren.elem,
            _ => break,
        }
    }

    match *ty {
        Type::Path(ref ty) => {
            let last_segment = ty.path.segments.iter().next_back().unwrap();
            match last_segment.arguments {
                AngleBracketed(ref args) if last_segment.ident == "Option" => {
                    match args.args.iter().next_back() {
                        Some(GenericArgument::Type(ty)) => Some(ty),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn ty_for_foreign_derive(item: &DeriveInput, model: &Model) -> Result<Type> {
    if model.foreign_derive {
        match item.data {
            Data::Struct(ref body) => match body.fields.iter().next() {
                Some(field) => Ok(field.ty.clone()),
                None => Err(syn::Error::new(
                    proc_macro2::Span::mixed_site(),
                    "foreign_derive requires at least one field",
                )),
            },
            _ => Err(syn::Error::new(
                proc_macro2::Span::mixed_site(),
                "foreign_derive can only be used with structs",
            )),
        }
    } else {
        let ident = &item.ident;
        let (_, ty_generics, ..) = item.generics.split_for_impl();
        Ok(parse_quote!(#ident #ty_generics))
    }
}

pub fn camel_to_snake(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    result.push_str(&name[..1].to_lowercase());
    for character in name[1..].chars() {
        if character.is_uppercase() {
            result.push('_');
            for lowercase in character.to_lowercase() {
                result.push(lowercase);
            }
        } else {
            result.push(character);
        }
    }
    result
}
