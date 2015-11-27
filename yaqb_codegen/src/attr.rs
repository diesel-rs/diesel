use syntax::ast;
use syntax::attr::AttrMetaMethods;
use syntax::ext::base::ExtCtxt;
use syntax::ptr::P;
use syntax::parse::token::str_to_ident;

pub struct Attr {
    pub column_name: ast::Ident,
    pub field_name: Option<ast::Ident>,
    pub ty: P<ast::Ty>,
}

impl Attr {
    pub fn from_struct_field(cx: &mut ExtCtxt, field: &ast::StructField) -> Option<Self> {
        let field_name = field.node.ident();
        let column_name = field.node.attrs.iter().filter_map(|attr| {
            if attr.check_name("column_name") {
                attr.value_str().map(|name| {
                    str_to_ident(&name)
                }).or_else(|| {
                    cx.span_err(attr.span(),
                        r#"`column_name` must be in the form `#[column_name="something"]`"#);
                    None
                })
            } else {
                None
            }
        }).nth(0);
        let ty = field.node.ty.clone();

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
                    r#"Field must be named or annotated with #[column_name="something"]"#);
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
            ast::ItemStruct(ref variant_data, ref generics) => {
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
}
