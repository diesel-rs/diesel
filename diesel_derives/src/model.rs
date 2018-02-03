use syn;

use attr::Attr;
use util::*;

pub struct Model {
    pub ty: syn::Ty,
    pub attrs: Vec<Attr>,
    pub name: syn::Ident,
    pub generics: syn::Generics,
    pub primary_key_names: Vec<syn::Ident>,
    table_name_from_annotation: Option<syn::Ident>,
}

impl Model {
    pub fn from_item(item: &syn::DeriveInput, derived_from: &str) -> Result<Self, String> {
        let attrs = match item.body {
            syn::Body::Enum(..) => {
                return Err(format!(
                    "#[derive({})] cannot be used with enums",
                    derived_from
                ))
            }
            syn::Body::Struct(ref fields) => attrs_from_struct_body(fields),
        };
        let ty = struct_ty(item.ident.clone(), &item.generics);
        let name = item.ident.clone();
        let generics = item.generics.clone();
        let primary_key_names = list_value_of_attr_with_name(&item.attrs, "primary_key")
            .map(|v| v.into_iter().cloned().collect())
            .unwrap_or_else(|| vec![syn::Ident::new("id")]);
        let table_name_from_annotation = ident_value_of_attr_with_name(&item.attrs, "table_name");

        Ok(Model {
            ty: ty,
            attrs: attrs,
            name: name,
            generics: generics,
            primary_key_names: primary_key_names,
            table_name_from_annotation: table_name_from_annotation,
        })
    }

    pub fn table_name(&self) -> syn::Ident {
        self.table_name_from_annotation
            .clone()
            .unwrap_or_else(|| syn::Ident::new(infer_table_name(self.name.as_ref())))
    }

    pub fn has_table_name_annotation(&self) -> bool {
        self.table_name_from_annotation.is_some()
    }
}

pub fn infer_association_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    result.push_str(&name[..1].to_lowercase());
    for character in name[1..].chars() {
        if character.is_uppercase() {
            result.push('_');
            for lowercase in character.to_lowercase() {
                result.push(lowercase);
            }
        } else {
            result.push(character);
        }
    }
    result
}

fn infer_table_name(name: &str) -> String {
    let mut result = infer_association_name(name);
    result.push('s');
    result
}

fn attrs_from_struct_body(body: &syn::VariantData) -> Vec<Attr> {
    body.fields()
        .into_iter()
        .enumerate()
        .map(Attr::from_struct_field)
        .collect()
}
