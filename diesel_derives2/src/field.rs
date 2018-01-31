use syn;
use quote;

use meta::*;

pub struct Field {
    pub ty: syn::Type,
    pub name: FieldName,
    column_name_from_attribute: Option<syn::Ident>,
}

impl Field {
    pub fn from_struct_field(field: &syn::Field, index: usize) -> Self {
        let column_name_from_attribute = MetaItem::with_name(&field.attrs, "column_name")
            .ok()
            .map(|m| m.expect_ident_value());
        let name = match field.ident {
            Some(x) => FieldName::Named(x),
            None => FieldName::Unnamed(index.into()),
        };

        Self {
            ty: field.ty.clone(),
            column_name_from_attribute,
            name,
        }
    }

    pub fn column_name(&self) -> syn::Ident {
        self.column_name_from_attribute
            .unwrap_or_else(|| match self.name {
                FieldName::Named(x) => x,
                _ => panic!("All fields of tuple structs must be annotated with `#[column_name]`"),
            })
    }
}

pub enum FieldName {
    Named(syn::Ident),
    Unnamed(syn::Index),
}

impl quote::ToTokens for FieldName {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        match *self {
            FieldName::Named(x) => x.to_tokens(tokens),
            FieldName::Unnamed(ref x) => x.to_tokens(tokens),
        }
    }
}
