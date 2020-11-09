use proc_macro2;
use syn;

use field::Field;
use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let model = Model::from_item(&item)?;

    let struct_name = &item.ident;
    let field_ty = model
        .fields()
        .iter()
        .map(Field::ty_for_deserialize)
        .collect::<Result<Vec<_>, _>>()?;
    let field_ty = &field_ty;
    let build_expr = model.fields().iter().enumerate().map(|(i, f)| {
        let i = syn::Index::from(i);
        f.name.assign(parse_quote!(row.#i.try_into()?))
    });

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, Queryable};
        use std::convert::TryInto;

        impl #impl_generics Queryable for #struct_name #ty_generics
            #where_clause
        {
            type Row = (#(#field_ty,)*);

            fn build(row: Self::Row) -> deserialize::Result<Self> {
                Ok(Self {
                    #(#build_expr,)*
                })
            }
        }
    }))
}
