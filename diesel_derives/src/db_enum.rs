use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, LitByteStr, Result};

use crate::attrs::{parse_attributes, EnumAttr};

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let error_message = "this derive can only be used on enums with only unit-variants";
    let variants = match &item.data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::mixed_site(),
                error_message,
            ));
        }
    };

    let mut from_bytes_arms = Vec::with_capacity(variants.len());
    let mut as_bytes_arms = Vec::with_capacity(variants.len());
    for v in variants {
        if !v.fields.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::mixed_site(),
                error_message,
            ));
        }

        let variant_as_byte_str = LitByteStr::new(v.ident.to_string().as_bytes(), v.span());
        let variant_name = &v.ident;

        from_bytes_arms.push(quote! {#variant_as_byte_str => Some(Self::#variant_name)});
        as_bytes_arms.push(quote! {Self::#variant_name => #variant_as_byte_str})
    }

    let mut postgres = false;
    let mut mysql = false;

    for attr in parse_attributes(&item.attrs)? {
        match attr.item {
            EnumAttr::Backend(_, backends) => {
                for backend in backends {
                    let Some(backend) = backend.path.segments.last() else {
                        return Err(syn::Error::new(
                            proc_macro2::Span::mixed_site(),
                            "this derive requires at least one database backend to be specified",
                        ));
                    };

                    match backend.ident.to_string().as_str() {
                        "Pg" => {
                            postgres = true;
                        }
                        "Mysql" => {
                            mysql = true;
                        }
                        _ => {
                            return Err(syn::Error::new(
                                proc_macro2::Span::mixed_site(),
                                "this derive only supports the Postgres and Mysql backends",
                            ));
                        }
                    }
                }
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let enum_name = &item.ident;
    let impl_from_and_to_bytes = quote! {
        impl #impl_generics #enum_name #ty_generics #where_clause {
            fn from_bytes(bytes: &[u8]) -> Option<Self> {
                match bytes {
                    #(#from_bytes_arms),*
                    _ => None
                }
            }

            fn as_bytes(&self) -> &[u8] {
                match self {
                    #(#as_bytes_arms),*
                }
            }
        }
    };

    let mut impls = Vec::with_capacity(2);

    if postgres {
        impls.push(
            quote! {
                impl #impl_generics ::diesel::FromSql<::diesel::sql_types::Text, ::diesel::pg::Pg> for #enum_name #ty_generics #where_clause {
                    fn from_sql(value: ::diesel::pg::PgValue<'_>) -> ::diesel::deserialize::Result<Self> {
                        Self::from_bytes(value.as_bytes())?
                    }
                }

                impl #impl_generics ::diesel::ToSql<::diesel::sql_types::Text, ::diesel::pg::Pg> for #enum_name #ty_generics #where_clause {
                    fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                        out.write_all(self.as_bytes())?;
                        Ok(IsNull::No)
                    }
                }
            }
        );
    }

    Ok(quote! {
        #impl_from_and_to_bytes
        #(#impls)*
    })
}
