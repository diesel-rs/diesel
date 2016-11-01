use syn;
use quote;

use constants::{attrs, custom_attrs, custom_derives};
use model::Model;

pub fn derive_insertable(item: syn::MacroInput) -> quote::Tokens {
    let model = t!(Model::from_item(&item, custom_derives::INSERTABLE));

    if !model.has_table_name_annotation() {
        panic!(r#"`#[{}({})]` requires the struct to be annotated \
            with `#[{}="something"]`"#, attrs::DERIVE, custom_derives::INSERTABLE,
            custom_attrs::TABLE_NAME);
    }

    if !model.generics.ty_params.is_empty() {
        panic!("`#[{}({})]` does not support generic types", attrs::DERIVE,
            custom_derives::INSERTABLE);
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
            lifetimes = (#(lifetimes),*),
        ),
        fields = [#(fields)*],
    })
}
