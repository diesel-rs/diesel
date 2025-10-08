use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    spanned::Spanned, Data, DeriveInput, Ident, ImplGenerics, LitByteStr, Result, TypeGenerics,
    Variant, WhereClause,
};

use crate::{
    attrs::{parse_attributes, AttributeSpanWrapper, EnumAttr},
    util::wrap_in_dummy_mod,
};

const ERROR_MESSAGE: &str = "this derive can only be used on enums with exclusively unit-variants";

fn validate_variant_has_no_fields(enum_variant: &Variant) -> Result<()> {
    if !enum_variant.fields.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::mixed_site(),
            ERROR_MESSAGE,
        ));
    }

    Ok(())
}

trait AsByteStringLiteral {
    fn as_byte_string_literal(&self) -> LitByteStr;
}

impl AsByteStringLiteral for Variant {
    fn as_byte_string_literal(&self) -> LitByteStr {
        LitByteStr::new(self.ident.to_string().as_bytes(), self.span())
    }
}

fn from_bytes_method_body(enum_variant: &Variant) -> Result<TokenStream> {
    validate_variant_has_no_fields(enum_variant)?;

    let variant_as_byte_string = enum_variant.as_byte_string_literal();
    let variant_name = &enum_variant.ident;

    Ok(quote! {#variant_as_byte_string => Some(Self::#variant_name)})
}

fn as_bytes_method_body(enum_variant: &Variant) -> Result<TokenStream> {
    validate_variant_has_no_fields(enum_variant)?;

    let variant_as_byte_string = enum_variant.as_byte_string_literal();
    let variant_name = &enum_variant.ident;

    Ok(quote! {Self::#variant_name => #variant_as_byte_string})
}

fn parse_backends(enum_attr: &AttributeSpanWrapper<EnumAttr>) -> Result<Vec<String>> {
    let mut parsed_backends = Vec::new();

    match &enum_attr.item {
        EnumAttr::Backend(_, backends) => {
            for backend in backends {
                let Some(backend) = backend.path.segments.last() else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::mixed_site(),
                        "this derive requires at least one database backend to be specified",
                    ));
                };

                parsed_backends.push(backend.ident.to_string())
            }
        }
    }

    Ok(parsed_backends)
}

fn impl_from_sql(
    enum_name: &Ident,
    (impl_generics, ty_generics, where_clause): (ImplGenerics, TypeGenerics, Option<&WhereClause>),
    backend: TokenStream,
    value_type: TokenStream,
) -> TokenStream {
    quote! {
        impl #impl_generics ::diesel::deserialize::FromSql<::diesel::sql_types::Text, #backend> for #enum_name #ty_generics #where_clause {
            fn from_sql(value: #value_type) -> ::diesel::deserialize::Result<Self> {
                Ok(Self::from_bytes(value.as_bytes()).unwrap())
            }
        }
    }
}

fn impl_to_sql(
    enum_name: &Ident,
    (impl_generics, ty_generics, where_clause): (ImplGenerics, TypeGenerics, Option<&WhereClause>),
    backend: TokenStream,
) -> TokenStream {
    quote! {
        impl #impl_generics ::diesel::serialize::ToSql<::diesel::sql_types::Text, #backend> for #enum_name #ty_generics #where_clause {
            fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, #backend>) -> ::diesel::serialize::Result {
                use ::std::io::Write;

                out.write_all(self.as_bytes())?;
                Ok(::diesel::serialize::IsNull::No)
            }
        }
    }
}

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let enum_variants = match &item.data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::mixed_site(),
                ERROR_MESSAGE,
            ));
        }
    };

    let mut from_bytes_arms = Vec::with_capacity(enum_variants.len());
    let mut as_bytes_arms = Vec::with_capacity(enum_variants.len());
    for v in enum_variants {
        from_bytes_arms.push(from_bytes_method_body(v)?);
        as_bytes_arms.push(as_bytes_method_body(v)?)
    }

    let mut postgres = false;
    let mut mysql = false;

    for attr in parse_attributes(&item.attrs)? {
        let backends = parse_backends(&attr)?;
        if backends.contains(&"Pg".to_string()) {
            postgres = true;
        }

        if backends.contains(&"Mysql".to_string()) {
            mysql = true;
        }
    }

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let enum_name = &item.ident;
    let impl_from_and_to_bytes = quote! {
        impl #impl_generics #enum_name #ty_generics #where_clause {
            fn from_bytes(bytes: &[u8]) -> Option<Self> {
                match bytes {
                    #(#from_bytes_arms),*,
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

    // can remove type annotation once this Vec is filled
    let mut impls: Vec<TokenStream> = Vec::with_capacity(2);

    Ok(wrap_in_dummy_mod(quote! {
        #impl_from_and_to_bytes
        #(#impls)*
    }))
}
