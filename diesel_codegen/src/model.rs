use syn;

use attr::Attr;
use util::struct_ty;

pub struct Model {
    pub ty: syn::Ty,
    pub attrs: Vec<Attr>,
    pub name: syn::Ident,
    pub generics: syn::Generics,
}

impl Model {
    pub fn from_item(item: &syn::MacroInput, derived_from: &str) -> Result<Self, String> {
        let fields = match item.body {
            syn::Body::Enum(..) => return Err(format!(
                "#[derive({})] cannot be used with enums", derived_from)),
            syn::Body::Struct(ref fields) => fields.fields(),
        };
        let attrs = fields.into_iter().map(Attr::from_struct_field).collect();
        let ty = struct_ty(item.ident.clone(), &item.generics);
        let name = item.ident.clone();
        let generics = item.generics.clone();

        Ok(Model {
            ty: ty,
            attrs: attrs,
            name: name,
            generics: generics,
        })
    }
}
