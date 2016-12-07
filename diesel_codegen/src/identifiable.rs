use quote::Tokens;
use syn;

use model::Model;

pub fn derive_identifiable(item: syn::MacroInput) -> Tokens {
    let model = t!(Model::from_item(&item, "Identifiable"));
    let table_name = model.table_name();
    let struct_ty = &model.ty;
    let lifetimes = model.generics.lifetimes;
    let fields = model.attrs;
    if !fields.iter().any(|f| f.field_name == Some(syn::Ident::new("id"))) {
        panic!("Could not find a field named `id` on `{}`", &model.name);
    }

    quote!(impl_Identifiable! {
        (
            table_name = #table_name,
            struct_ty = #struct_ty,
            lifetimes = (#(#lifetimes),*),
        ),
        fields = [#(#fields)*],
    })
}
