use quote::Tokens;
use syn;

use constants::{custom_derives, syntax};
use model::Model;

pub fn derive_identifiable(item: syn::MacroInput) -> Tokens {
    let model = t!(Model::from_item(&item, custom_derives::IDENTIFIABLE));
    let table_name = model.table_name();
    let struct_ty = &model.ty;
    let fields = model.attrs;
    if !fields.iter().any(|f| f.field_name == Some(syn::Ident::new(syntax::ID))) {
        panic!("Could not find a field named `{}` on `{}`", syntax::ID, &model.name);
    }

    quote!(_Identifiable! {
        (
            table_name = #table_name,
            struct_ty = #struct_ty,
        ),
        fields = [#(fields)*],
    })
}
