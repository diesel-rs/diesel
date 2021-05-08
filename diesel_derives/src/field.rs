use proc_macro2::{self, Ident, Span};
use quote::ToTokens;
use std::borrow::Cow;
use syn;
use syn::spanned::Spanned;

use meta::*;
use util::*;

pub struct Field {
    pub ty: syn::Type,
    pub name: FieldName,
    pub span: Span,
    pub sql_type: Option<syn::Type>,
    pub flags: MetaItem,
    column_name_from_attribute: Option<MetaItem>,
}

impl Field {
    pub fn from_struct_field(field: &syn::Field, index: usize) -> Self {
        let column_name_from_attribute = MetaItem::with_name(&field.attrs, "column_name");
        let name = match field.ident.clone() {
            Some(mut x) => {
                // https://github.com/rust-lang/rust/issues/47983#issuecomment-362817105
                let span = x.span();
                x.set_span(fix_span(span, Span::call_site()));
                FieldName::Named(x)
            }
            None => FieldName::Unnamed(syn::Index {
                index: index as u32,
                // https://github.com/rust-lang/rust/issues/47312
                span: Span::call_site(),
            }),
        };
        let sql_type = MetaItem::with_name(&field.attrs, "sql_type")
            .and_then(|m| m.ty_value().map_err(Diagnostic::emit).ok());
        let flags = MetaItem::with_name(&field.attrs, "diesel")
            .unwrap_or_else(|| MetaItem::empty("diesel"));
        let span = field.span();

        Self {
            ty: field.ty.clone(),
            column_name_from_attribute,
            name,
            sql_type,
            flags,
            span,
        }
    }

    pub fn column_name_ident(&self) -> syn::Ident {
        self.column_name_from_attribute
            .as_ref()
            .map(|m| m.expect_ident_value())
            .unwrap_or_else(|| match self.name {
                FieldName::Named(ref x) => x.clone(),
                _ => {
                    self.span
                        .error(
                            "All fields of tuple structs must be annotated with `#[column_name]`",
                        )
                        .emit();
                    Ident::new("unknown_column", self.span)
                }
            })
    }

    pub fn column_name_str(&self) -> String {
        self.column_name_from_attribute
            .as_ref()
            .map(|m| {
                m.str_value().unwrap_or_else(|e| {
                    e.emit();
                    m.name().segments.first().unwrap().ident.to_string()
                })
            })
            .unwrap_or_else(|| match self.name {
                FieldName::Named(ref x) => x.to_string(),
                _ => {
                    self.span
                        .error(
                            "All fields of tuple structs must be annotated with `#[column_name]`",
                        )
                        .emit();
                    "unknown_column".to_string()
                }
            })
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.has_flag(flag)
    }

    pub fn ty_for_serialize(&self) -> Result<Option<syn::Type>, Diagnostic> {
        if let Some(meta) = self.flags.nested_item("serialize_as")? {
            let ty = meta.ty_value()?;
            Ok(Some(ty))
        } else {
            Ok(None)
        }
    }

    pub fn ty_for_deserialize(&self) -> Result<Cow<syn::Type>, Diagnostic> {
        if let Some(meta) = self.flags.nested_item("deserialize_as")? {
            meta.ty_value().map(Cow::Owned)
        } else {
            Ok(Cow::Borrowed(&self.ty))
        }
    }
}

pub enum FieldName {
    Named(syn::Ident),
    Unnamed(syn::Index),
}

impl FieldName {
    pub fn assign(&self, expr: syn::Expr) -> syn::FieldValue {
        let span = self.span();
        // Parens are to work around https://github.com/rust-lang/rust/issues/47311
        let tokens = quote_spanned!(span=> #self: (#expr));
        parse_quote!(#tokens)
    }

    pub fn access(&self) -> proc_macro2::TokenStream {
        let span = self.span();
        // Span of the dot is important due to
        // https://github.com/rust-lang/rust/issues/47312
        quote_spanned!(span=> .#self)
    }

    pub fn span(&self) -> Span {
        match *self {
            FieldName::Named(ref x) => x.span(),
            FieldName::Unnamed(ref x) => x.span,
        }
    }
}

impl ToTokens for FieldName {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match *self {
            FieldName::Named(ref x) => x.to_tokens(tokens),
            FieldName::Unnamed(ref x) => x.to_tokens(tokens),
        }
    }
}
