use quote::Tokens;
use syn;

use model::Model;

pub fn derive_queryable(item: syn::MacroInput) -> Tokens {
    let model = t!(Model::from_item(&item, "Queryable"));

    let struct_ty = &model.ty;
    let struct_name = &model.name;
    let ty_params = &model.generics.ty_params;
    let attrs = model.attrs;
    let lifetimes = &model.generics.lifetimes;

    quote!(_Queryable! {
        (
            struct_name = #struct_name,
            struct_ty = #struct_ty,
            generics = (#(ty_params),*),
            lifetimes = (#(lifetimes),*),
        ),
        fields = [#(attrs)*],
    })
}
