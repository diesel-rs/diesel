use quote::Tokens;
use syn;

use model::Model;

pub fn derive_identifiable(item: syn::MacroInput) -> Tokens {
    let model = t!(Model::from_item(&item, "Identifiable"));
    let table_name = model.table_name();
    let struct_ty = &model.ty;
    let lifetimes = model.generics.lifetimes;
    let primary_key_names = model.primary_key_names;
    let fields = model.attrs.as_slice();
    for pk in &primary_key_names {
        if !fields.iter().any(|f| f.field_name.as_ref() == Some(pk)) {
            panic!("Could not find a field named `{}` on `{}`", pk, &model.name);
        }
    }

    quote!(impl_Identifiable! {
        (
            table_name = #table_name,
            primary_key_names = (#(#primary_key_names),*),
            struct_ty = #struct_ty,
            lifetimes = (#(#lifetimes),*),
        ),
        fields = [#(#fields)*],
    })
}
