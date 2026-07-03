use proc_macro2::{Span, TokenStream};
use syn::{Data, Ident, Result, spanned::Spanned};

use crate::attrs::AttributeSpanWrapper;
use crate::util::wrap_in_dummy_mod;

const ERROR_MESSAGE: &str = "this derive can only be used on enums with exclusively unit-variants";

pub fn derive(item: DeriveEnumInput) -> Result<TokenStream> {
    let mut to_sql_impls = Vec::new();
    let mut from_sql_impls = Vec::new();
    for tpe in &item.sql_type_attrs {
        let span = tpe.attribute_span;
        let tpe = &tpe.item;
        let sql_type = quote::quote_spanned! {span=> #tpe};
        let enum_name = &item.ident;
        let has_explicit_discriminant = item.has_explicit_discriminants;
        let variants = item
            .variants
            .iter()
            .map(EnumVariant::as_diesel_enum_variant)
            .collect::<Vec<_>>();
        let variant_constructor = item.variants.iter().enumerate().map(|(idx, v)| {
            let span = v.span;
            let ident = &v.rust_name;
            quote::quote_spanned! {span=> #idx => Ok(Self::#ident)}
        });
        let variant_to_enum_variant_mapping = item.variants.iter().map(|v| {
            let span = v.span;
            let ident = &v.rust_name;
            let enum_variant = v.as_diesel_enum_variant();
            quote::quote_spanned! {span=> Self::#ident => &#enum_variant}
        });

        from_sql_impls.push(quote::quote! {
            impl<__DB> diesel::deserialize::FromSql<#sql_type, __DB> for #enum_name
            where
                __DB: diesel::backend::Backend,
                #sql_type: diesel::sql_types::EnumSqlType<#has_explicit_discriminant, __DB>,
                <#sql_type as diesel::sql_types::EnumSqlType<#has_explicit_discriminant, __DB>>::Strategy: diesel::internal::derives::enum_::EnumMapping<__DB>,
            {
                fn from_sql(value: <__DB as diesel::backend::Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
                    const VARIANTS: &[diesel::internal::derives::enum_::EnumVariant] = &[#(#variants,)*];
                    let idx = <<#sql_type as diesel::sql_types::EnumSqlType<#has_explicit_discriminant, __DB>>::Strategy as diesel::internal::derives::enum_::EnumMapping<__DB>>::map_from_database_value(
                        value,
                        stringify!(#enum_name),
                        VARIANTS
                    )?;
                    match idx {
                        #(#variant_constructor,)*
                        _ => unreachable!("We construct all relevant variants"),
                    }
                }
            }
        });

        to_sql_impls.push(quote::quote! {
            impl<__DB> diesel::serialize::ToSql<#sql_type, __DB> for #enum_name
            where
                __DB: diesel::backend::Backend,
                #sql_type: diesel::sql_types::EnumSqlType<#has_explicit_discriminant, __DB>,
                <#sql_type as diesel::sql_types::EnumSqlType<#has_explicit_discriminant, __DB>>::Strategy: diesel::internal::derives::enum_::EnumMapping<__DB>,
            {
                fn to_sql<'b>(&'b self, output: &mut diesel::serialize::Output<'b, '_, __DB>) -> diesel::serialize::Result {
                    let variant = match self {
                        #(#variant_to_enum_variant_mapping,)*
                    };
                    <<#sql_type as diesel::sql_types::EnumSqlType<#has_explicit_discriminant, __DB>>::Strategy as diesel::internal::derives::enum_::EnumMapping<__DB>>::map_to_database_value(
                        output,
                        variant
                    )
                }
            }
        });
    }

    let struct_ty = syn::Type::Path(syn::TypePath {
        qself: None,
        path: item.ident.into(),
    });
    let sql_types = item
        .sql_type_attrs
        .iter()
        .map(|v| syn::Type::Path(v.item.clone()))
        .collect::<Vec<_>>();
    let as_expression_impl = super::as_expression::derive_inner(
        sql_types,
        syn::Generics::default(),
        struct_ty.clone(),
        false,
        false,
    )?;
    let from_sql_row_impl = super::from_sql_row::derive_inner(struct_ty, syn::Generics::default())?;

    Ok(wrap_in_dummy_mod(quote::quote! {
        #(#from_sql_impls)*
        #(#to_sql_impls)*

        #as_expression_impl
        #from_sql_row_impl
    }))
}

pub struct DeriveEnumInput {
    sql_type_attrs: Vec<AttributeSpanWrapper<syn::TypePath>>,
    ident: syn::Ident,
    has_explicit_discriminants: bool,
    variants: Vec<EnumVariant>,
}

pub struct EnumVariant {
    span: Span,
    discriminant: i128,
    rust_name: Ident,
    sql_name: String,
}

impl EnumVariant {
    fn as_diesel_enum_variant(&self) -> TokenStream {
        let Self {
            span,
            discriminant,
            rust_name,
            sql_name,
        } = self;
        let span = *span;
        quote::quote_spanned! {span=>
                diesel::internal::derives::enum_::EnumVariant {
                    discriminant: #discriminant,
                    rust_name: stringify!(#rust_name),
                    sql_name: #sql_name,
                }
        }
    }
}

impl syn::parse::Parse for DeriveEnumInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let input = input.parse::<syn::DeriveInput>()?;
        let input_span = input.span();
        if !input.generics.params.is_empty() {
            return Err(syn::Error::new(input.span(), ERROR_MESSAGE));
        }
        let enum_ = match input.data {
            Data::Enum(data_enum) => data_enum,
            _ => {
                return Err(syn::Error::new(input.span(), ERROR_MESSAGE));
            }
        };
        let attrs = crate::attrs::parse_attributes::<crate::attrs::StructAttr>(&input.attrs)?;
        let rename_all = attrs.iter().find_map(|a| {
            if let crate::attrs::StructAttr::RenameAll(_, r) = &a.item {
                Some(r)
            } else {
                None
            }
        });

        let mut has_explicit_discriminants = true;
        let mut variants = Vec::with_capacity(enum_.variants.len());
        for (idx, v) in enum_.variants.iter().enumerate() {
            if !v.fields.is_empty() {
                return Err(syn::Error::new(v.span(), ERROR_MESSAGE));
            }
            has_explicit_discriminants = has_explicit_discriminants && v.discriminant.is_some();
            let discriminant = v
                .discriminant
                .as_ref()
                .map(|(_, v)| {
                    let (f, l) = match v {
                        syn::Expr::Lit(l) => (1, l),
                        syn::Expr::Unary(syn::ExprUnary {
                            op: syn::UnOp::Neg(_),
                            expr,
                            ..
                        }) => {
                            if let syn::Expr::Lit(l) = &**expr {
                                (-1, l)
                            } else {
                                return Err(syn::Error::new(
                                    v.span(),
                                    "expected a literal expression, but got something else",
                                ));
                            }
                        }
                        _ => {
                            return Err(syn::Error::new(
                                v.span(),
                                "expected a literal expression, but got something else",
                            ));
                        }
                    };
                    let syn::Lit::Int(i) = &l.lit else {
                        return Err(syn::Error::new(
                            l.span(),
                            "expected a integer literal expression, but got something else",
                        ));
                    };
                    Ok(f * i.base10_parse::<i128>()?)
                })
                .transpose()?
                .unwrap_or(idx as i128);
            let rust_name = v.ident.clone();
            let attrs = crate::attrs::parse_attributes::<crate::attrs::FieldAttr>(&v.attrs)?;
            let rename_attr = attrs.iter().find_map(|a| {
                if let crate::attrs::FieldAttr::Rename(_, r) = &a.item {
                    Some(r.value())
                } else {
                    None
                }
            });
            let sql_name = rename_attr
                .or_else(|| Some(rename_all?.apply_case_to_enum_variant(rust_name.to_string())))
                .unwrap_or_else(|| rust_name.to_string());
            variants.push(EnumVariant {
                span: v.span(),
                discriminant,
                rust_name,
                sql_name,
            });
        }

        let sql_type_attrs = attrs
            .into_iter()
            .filter_map(|a: AttributeSpanWrapper<crate::attrs::StructAttr>| {
                if let crate::attrs::StructAttr::SqlType(_, path) = a.item {
                    Some(crate::attrs::AttributeSpanWrapper {
                        item: path,
                        attribute_span: a.attribute_span,
                        ident_span: a.ident_span,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if sql_type_attrs.is_empty() {
            return Err(syn::Error::new(
                input_span,
                "no `#[diesel(sql_type = ...)]` attribute provided",
            ));
        }

        Ok(Self {
            sql_type_attrs,
            ident: input.ident,
            has_explicit_discriminants,
            variants,
        })
    }
}
