use quote;
use syn;

use constants::field_attrs::COLUMN_NAME;
use constants::field_types::*;
use constants::syntax;
use util::{ident_value_of_attr_with_name, is_option_ty};

pub struct Attr {
    pub column_name: Option<syn::Ident>,
    pub field_name: Option<syn::Ident>,
    pub ty: syn::Ty,
}

impl Attr {
    pub fn from_struct_field(field: &syn::Field) -> Self {
        let field_name = field.ident.clone();
        let column_name = ident_value_of_attr_with_name(&field.attrs, COLUMN_NAME)
            .map(Clone::clone)
            .or_else(|| field_name.clone());
        let ty = field.ty.clone();

        Attr {
            column_name: column_name,
            field_name: field_name,
            ty: ty,
        }
    }

    fn field_kind(&self) -> &str {
        if is_option_ty(&self.ty) {
            OPTION
        } else if self.column_name.is_none() && self.field_name.is_none() {
            BARE
        } else {
            REGULAR
        }
    }
}

impl quote::ToTokens for Attr {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        tokens.append("{");
        if let Some(ref name) = self.field_name {
            tokens.append(&format!("{}: ", syntax::FIELD_NAME));
            name.to_tokens(tokens);
            tokens.append(", ");
        }
        if let Some(ref name) = self.column_name {
            tokens.append(&format!("{}: ", syntax::COLUMN_NAME));
            name.to_tokens(tokens);
            tokens.append(", ");
        }
        tokens.append(&format!("{}: ", syntax::FIELD_TY));
        self.ty.to_tokens(tokens);
        tokens.append(", ");
        tokens.append(&format!("{}: ", syntax::FIELD_KIND));
        tokens.append(self.field_kind());
        tokens.append(", ");
        tokens.append("}");
    }
}
