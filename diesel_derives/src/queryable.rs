use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, DeriveInput, Ident, Index, Result};

use crate::field::Field;
use crate::model::Model;
use crate::util::wrap_in_dummy_mod;

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, false, true)?;

    let struct_name = &item.ident;
    let field_ty = &model
        .fields()
        .iter()
        .map(Field::ty_for_deserialize)
        .collect::<Vec<_>>();
    let build_expr = model.fields().iter().enumerate().map(|(i, f)| {
        let field_name = &f.name;
        let i = Index::from(i);
        quote!(#field_name: row.#i.try_into()?)
    });
    let sql_type = &(0..model.fields().len())
        .map(|i| {
            let i = Ident::new(&format!("__ST{i}"), Span::call_site());
            quote!(#i)
        })
        .collect::<Vec<_>>();

    let (_, ty_generics, _) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));
    for id in 0..model.fields().len() {
        let ident = Ident::new(&format!("__ST{id}"), Span::call_site());
        generics.params.push(parse_quote!(#ident));
    }
    {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!((#(#field_ty,)*): FromStaticSqlRow<(#(#sql_type,)*), __DB>));
    }
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, FromStaticSqlRow, Queryable};
        use diesel::row::{Row as _, Field as _};
        use std::convert::TryInto;

        impl #impl_generics Queryable<(#(#sql_type,)*), __DB> for #struct_name #ty_generics
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
