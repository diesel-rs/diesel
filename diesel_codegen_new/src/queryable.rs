use quote::Tokens;
use syn;

use model::Model;

pub fn derive_queryable(item: syn::Item) -> Tokens {
    let model = match Model::from_item(&item) {
        Ok(m) => m,
        Err(e) => panic!("#[derive(Queryable)] {}", e),
    };

    let struct_ty = &model.ty;
    let struct_name = &model.name;
    let ty_params = &model.generics.ty_params;
    let attrs = model.attrs;
    let lifetimes = &model.generics.lifetimes;

    quote!(Queryable! {
        (
            struct_name = #struct_name,
            struct_ty = #struct_ty,
            generics = (#(ty_params),*),
            lifetimes = (#(lifetimes),*),
        ),
        fields = [#(attrs)*],
    })
}
