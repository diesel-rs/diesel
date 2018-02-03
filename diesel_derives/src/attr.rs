use quote;
use syn;

use util::*;

#[derive(Debug)]
pub struct Attr {
    column_name: Option<syn::Ident>,
    field_name: Option<syn::Ident>,
    sql_type: Option<syn::Ty>,
    flags: Vec<syn::Ident>,
    pub ty: syn::Ty,
    pub field_position: syn::Ident,
}

impl Attr {
    pub fn from_struct_field((index, field): (usize, &syn::Field)) -> Self {
        let field_name = field.ident.clone();
        let column_name = ident_value_of_attr_with_name(&field.attrs, "column_name")
            .or_else(|| field_name.clone());
        let ty = field.ty.clone();
        let sql_type = ty_value_of_attr_with_name(&field.attrs, "sql_type");
        let flags = list_value_of_attr_with_name(&field.attrs, "diesel")
            .unwrap_or_else(Vec::new)
            .into_iter()
            .cloned()
            .collect();

        Attr {
            column_name,
            field_name,
            sql_type,
            ty,
            flags,
            field_position: index.to_string().into(),
        }
    }

    fn field_kind(&self) -> &str {
        if is_option_ty(&self.ty) {
            "option"
        } else if self.column_name.is_none() && self.field_name.is_none() {
            "bare"
        } else {
            "regular"
        }
    }
}

impl quote::ToTokens for Attr {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        tokens.append("{");
        if let Some(ref name) = self.field_name {
            tokens.append("field_name: ");
            name.to_tokens(tokens);
            tokens.append(", ");
        }
        if let Some(ref name) = self.column_name {
            tokens.append("column_name: ");
            name.to_tokens(tokens);
            tokens.append(", ");
        }
        tokens.append("field_ty: ");
        self.ty.to_tokens(tokens);
        tokens.append(", ");
        tokens.append("field_kind: ");
        tokens.append(self.field_kind());
        tokens.append(", ");
        tokens.append("inner_field_ty: ");
        inner_of_option_ty(&self.ty)
            .unwrap_or(&self.ty)
            .to_tokens(tokens);
        tokens.append(", ");
        tokens.append("}");
    }
}
