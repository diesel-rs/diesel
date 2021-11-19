use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Field as SynField, Ident, Index, Type};

use attrs::{parse_attributes, FieldAttr, SqlIdentifier};

pub struct Field {
    pub ty: Type,
    pub span: Span,
    pub name: FieldName,
    column_name: Option<SqlIdentifier>,
    pub sql_type: Option<Type>,
    pub serialize_as: Option<Type>,
    pub deserialize_as: Option<Type>,
    pub embed: bool,
}

impl Field {
    pub fn from_struct_field(field: &SynField, index: usize) -> Self {
        let SynField {
            ident, attrs, ty, ..
        } = field;

        let mut column_name = None;
        let mut sql_type = None;
        let mut serialize_as = None;
        let mut deserialize_as = None;
        let mut embed = false;

        for attr in parse_attributes(attrs) {
            match attr {
                FieldAttr::ColumnName(_, value) => column_name = Some(value),
                FieldAttr::SqlType(_, value) => sql_type = Some(value),
                FieldAttr::SerializeAs(_, value) => serialize_as = Some(value),
                FieldAttr::DeserializeAs(_, value) => deserialize_as = Some(value),
                FieldAttr::Embed(_) => embed = true,
            }
        }

        let name = match ident.clone() {
            Some(x) => FieldName::Named(x),
            None => FieldName::Unnamed(index.into()),
        };

        let span = match name {
            FieldName::Named(ref ident) => ident.span(),
            FieldName::Unnamed(_) => ty.span(),
        };

        Self {
            ty: ty.clone(),
            span,
            name,
            column_name,
            sql_type,
            serialize_as,
            deserialize_as,
            embed,
        }
    }

    pub fn column_name(&self) -> SqlIdentifier {
        self.column_name.clone().unwrap_or_else(|| match self.name {
            FieldName::Named(ref x) => x.into(),
            FieldName::Unnamed(ref x) => {
                abort!(
                    x,
                    "All fields of tuple structs must be annotated with `#[diesel(column_name)]`"
                );
            }
        })
    }

    pub fn ty_for_deserialize(&self) -> &Type {
        if let Some(value) = &self.deserialize_as {
            value
        } else {
            &self.ty
        }
    }
}

pub enum FieldName {
    Named(Ident),
    Unnamed(Index),
}

impl ToTokens for FieldName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            FieldName::Named(ref x) => x.to_tokens(tokens),
            FieldName::Unnamed(ref x) => x.to_tokens(tokens),
        }
    }
}
