use syn;

use meta::*;

pub struct Field {
    pub ty: syn::Type,
    column_name_from_attribute: Option<syn::Ident>,
    identifier: Identifier,
}

impl Field {
    pub fn from_struct_field(field: &syn::Field, index: usize) -> Self {
        let column_name_from_attribute = MetaItem::with_name(&field.attrs, "column_name")
            .ok()
            .map(|m| m.expect_ident_value());
        let identifier = match field.ident {
            Some(x) => Identifier::Named(x),
            None => Identifier::Unnamed(index.to_string().into()),
        };

        Self {
            ty: field.ty.clone(),
            column_name_from_attribute,
            identifier,
        }
    }

    pub fn column_name(&self) -> syn::Ident {
        self.column_name_from_attribute
            .unwrap_or_else(|| match self.identifier {
                Identifier::Named(x) => x,
                _ => panic!("All fields of tuple structs must be annotated with `#[column_name]`"),
            })
    }

    pub fn field_name(&self) -> syn::Ident {
        self.identifier.ident()
    }
}

#[derive(Debug)]
enum Identifier {
    Named(syn::Ident),
    Unnamed(syn::Ident),
}

impl Identifier {
    fn ident(&self) -> syn::Ident {
        use self::Identifier::*;

        match *self {
            Named(x) | Unnamed(x) => x,
        }
    }
}
