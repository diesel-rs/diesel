use syn;

use field::*;
use meta::*;

pub struct Model {
    pub name: syn::Ident,
    pub primary_key_names: Vec<syn::Ident>,
    table_name_from_attribute: Option<syn::Ident>,
    fields: Vec<Field>,
}

impl Model {
    pub fn from_item(item: &syn::DeriveInput) -> Self {
        let table_name_from_attribute = MetaItem::with_name(&item.attrs, "table_name")
            .ok()
            .map(|m| m.expect_ident_value());
        let primary_key_names = MetaItem::with_name(&item.attrs, "primary_key")
            .map(|m| m.expect_nested().map(|m| m.expect_word()).collect())
            .unwrap_or_else(|_| vec!["id".into()]);
        let fields = fields_from_item_data(&item.data);
        Self {
            name: item.ident,
            table_name_from_attribute,
            primary_key_names,
            fields,
        }
    }

    pub fn table_name(&self) -> syn::Ident {
        self.table_name_from_attribute
            .unwrap_or_else(|| infer_table_name(self.name.as_ref()).into())
    }

    pub fn dummy_const_name(&self, trait_name: &str) -> syn::Ident {
        let name = self.name.as_ref().to_uppercase();
        format!("_IMPL_{}_FOR_{}", trait_name, name).into()
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}

pub fn camel_to_snake(name: &str) -> String {
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
    let mut result = camel_to_snake(name);
    result.push('s');
    result
}

fn fields_from_item_data(data: &syn::Data) -> Vec<Field> {
    use syn::Data::*;

    let struct_data = match *data {
        Struct(ref d) => d,
        _ => panic!("This derive can only be used on structs"),
    };
    struct_data
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| Field::from_struct_field(f, i))
        .collect()
}
