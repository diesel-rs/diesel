use syn;
use quote;

use model::Model;

pub fn derive_insertable(item: syn::MacroInput) -> quote::Tokens {
    let model = t!(Model::from_item(&item, "Insertable"));

    if !model.has_table_name_annotation() {
        panic!(r#"`#[derive(Insertable)]` requires the struct to be annotated \
            with `#[table_name="something"]`"#);
    }

    if !model.generics.ty_params.is_empty() {
        panic!("`#[derive(Insertable)]` does not support generic types");
    }

    let struct_name = &model.name;
    let struct_ty = &model.ty;
    let table_name = &model.table_name();
    let lifetimes = model.generics.lifetimes;
    let fields = model.attrs;

    quote!(_Insertable! {
        (
            struct_name = #struct_name,
            table_name = #table_name,
            struct_ty = #struct_ty,
            lifetimes = (#(#lifetimes),*),
        ),
        fields = [#(#fields)*],
    })
}
