use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Data, DeriveInput, Ident, ImplGenerics, LitByteStr, Result, TypeGenerics, Variant, WhereClause,
    spanned::Spanned,
};

use crate::{model::Model, util::wrap_in_dummy_mod};

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

fn enum_variant_to_byte_string_literal(enum_variant: &Variant) -> LitByteStr {
    LitByteStr::new(
        enum_variant.ident.to_string().as_bytes(),
        enum_variant.span(),
    )
}

fn from_bytes_method_body(enum_variant: &Variant) -> TokenStream {
    let variant_as_byte_string = enum_variant_to_byte_string_literal(enum_variant);
    let variant_name = &enum_variant.ident;

    quote! {#variant_as_byte_string => Some(Self::#variant_name)}
}

fn as_bytes_method_body(enum_variant: &Variant) -> TokenStream {
    let variant_as_byte_string = enum_variant_to_byte_string_literal(enum_variant);
    let variant_name = &enum_variant.ident;

    quote! {Self::#variant_name => #variant_as_byte_string}
}

fn impl_from_sql(
    enum_name: &Ident,
    (impl_generics, ty_generics, where_clause): &(ImplGenerics, TypeGenerics, Option<&WhereClause>),
    from_sql_types: &[TokenStream],
    backend: &TokenStream,
    value_type: &TokenStream,
) -> TokenStream {
    let enum_name_as_str = enum_name.to_string();

    quote! {
        impl #impl_generics ::diesel::deserialize::FromSql<#(#from_sql_types)*, #backend> for #enum_name #ty_generics #where_clause {
            fn from_sql(value: #value_type) -> ::diesel::deserialize::Result<Self> {
                let raw_bytes = value.as_bytes();

                Self::from_bytes(raw_bytes).ok_or_else(|| format!("unable to convert bytes {:?} to {}", raw_bytes, #enum_name_as_str).into())
            }
        }
    }
}

fn impl_to_sql(
    enum_name: &Ident,
    (impl_generics, ty_generics, where_clause): &(ImplGenerics, TypeGenerics, Option<&WhereClause>),
    to_sql_types: &[TokenStream],
    backend: &TokenStream,
) -> TokenStream {
    quote! {
        impl #impl_generics ::diesel::serialize::ToSql<#(#to_sql_types)*, #backend> for #enum_name #ty_generics #where_clause {
            fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, #backend>) -> ::diesel::serialize::Result {
                use ::std::io::Write;

                out.write_all(self.as_bytes())?;
                Ok(::diesel::serialize::IsNull::No)
            }
        }
    }
}

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, true, false)?;

    let enum_variants = match &item.data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::mixed_site(),
                ERROR_MESSAGE,
            ));
        }
    };

    for v in enum_variants {
        validate_variant_has_no_fields(v)?;
    }

    let mut from_bytes_arms = Vec::with_capacity(enum_variants.len());
    let mut as_bytes_arms = Vec::with_capacity(enum_variants.len());
    for v in enum_variants {
        from_bytes_arms.push(from_bytes_method_body(v));
        as_bytes_arms.push(as_bytes_method_body(v))
    }

    let generics = item.generics.split_for_impl();

    let mut impls = Vec::new();
    let sql_types: Vec<_> = model
        .sql_types
        .iter()
        .map(syn::Type::to_token_stream)
        .collect();

    if cfg!(feature = "postgres") {
        let backend = quote! { ::diesel::pg::Pg };
        let value_type = quote! { ::diesel::pg::PgValue<'_> };

        impls.push(impl_from_sql(
            &item.ident,
            &generics,
            &sql_types,
            &backend,
            &value_type,
        ));
        impls.push(impl_to_sql(&item.ident, &generics, &sql_types, &backend));
    }

    if cfg!(feature = "mysql") {
        let backend = quote! { ::diesel::mysql::Mysql };
        let value_type = quote! { ::diesel::mysql::MysqlValue<'_> };

        impls.push(impl_from_sql(
            &item.ident,
            &generics,
            &sql_types,
            &backend,
            &value_type,
        ));
        impls.push(impl_to_sql(&item.ident, &generics, &sql_types, &backend));
    }

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let enum_name = &item.ident;
    let impl_from_and_to_bytes = quote! {
        impl #impl_generics #enum_name #ty_generics #where_clause {
            pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
                match bytes {
                    #(#from_bytes_arms),*,
                    _ => None
                }
            }

            pub fn as_bytes(&self) -> &[u8] {
                match self {
                    #(#as_bytes_arms),*
                }
            }
        }
    };

    let as_expression_impl = super::as_expression::derive(item.clone())?;
    let from_sql_row_impl = super::from_sql_row::derive(item)?;

    Ok(wrap_in_dummy_mod(quote! {
        #impl_from_and_to_bytes
        #(#impls)*
        #as_expression_impl
        #from_sql_row_impl
    }))
}
