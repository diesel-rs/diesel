use syntax::ast;
use syntax::ast::ItemKind;
use syntax::ext::base::ExtCtxt;
use syntax::parse::token::str_to_ident;
use syntax::ptr::P;
use syntax::tokenstream::TokenTree;

use util::{ident_value_of_attr_with_name, ty_param_of_option};

#[derive(Debug, PartialEq, Eq)]
pub struct Attr {
    pub column_name: ast::Ident,
    pub field_name: Option<ast::Ident>,
    pub ty: P<ast::Ty>,
}

impl Attr {
    pub fn from_struct_field(cx: &mut ExtCtxt, field: &ast::StructField) -> Option<Self> {
        let field_name = field.ident;
        let column_name =
            ident_value_of_attr_with_name(cx, &field.attrs, "column_name");
        let ty = field.ty.clone();

        match (column_name, field_name) {
            (Some(column_name), f) => Some(Attr {
                column_name: column_name,
                field_name: f,
                ty: ty,
            }),
            (None, Some(field_name)) => Some(Attr {
                column_name: field_name.clone(),
                field_name: Some(field_name),
                ty: ty,
            }),
            (None, None) => {
                cx.span_err(field.span,
                    r#"Field must be named or annotated with #[column_name(something)]"#);
                None
            }
        }
    }

    pub fn from_struct_fields(cx: &mut ExtCtxt, fields: &[ast::StructField])
        -> Option<Vec<Self>>
    {
        fields.iter().map(|f| Self::from_struct_field(cx, f)).collect()
    }

    pub fn from_item(cx: &mut ExtCtxt, item: &ast::Item)
        -> Option<(ast::Generics, Vec<Self>)>
    {
        match item.node {
            ItemKind::Struct(ref variant_data, ref generics) => {
                let fields = match *variant_data {
                    ast::VariantData::Struct(ref fields, _) => fields,
                    ast::VariantData::Tuple(ref fields, _) => fields,
                    _ => return None,
                };
                Self::from_struct_fields(cx, fields).map(|f| (generics.clone(), f))
            }
            _ => None
        }
    }

    pub fn to_stable_macro_tokens(&self, cx: &mut ExtCtxt) -> Vec<TokenTree> {
        let field_kind;
        let field_ty;
        let inner_field_ty;
        if let Some(option_ty) = ty_param_of_option(&self.ty) {
            field_kind = str_to_ident("option");
            field_ty = quote_tokens!(cx, Option<$option_ty>);
            inner_field_ty = quote_tokens!(cx, $option_ty);
        } else {
            let ty = &self.ty;
            field_kind = str_to_ident("regular");
            field_ty = quote_tokens!(cx, $ty);
            inner_field_ty = quote_tokens!(cx, $ty);
        }

        let column_name = self.column_name;
        match self.field_name {
            Some(field_name) => quote_tokens!(cx, {
                field_name: $field_name,
                column_name: $column_name,
                field_ty: $field_ty,
                field_kind: $field_kind,
                inner_field_ty: $inner_field_ty,
            }),
            None => quote_tokens!(cx, {
                column_name: $column_name,
                field_ty: $field_ty,
                field_kind: $field_kind,
                inner_field_ty: $inner_field_ty,
            }),
        }
    }
}
