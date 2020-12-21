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
    let sql_type = (0..model.fields().len())
        .map(|i| {
            let i = syn::Ident::new(&format!("__ST{}", i), proc_macro2::Span::call_site());
            quote!(#i)
        })
        .collect::<Vec<_>>();
    let sql_type = &sql_type;

    let (_, ty_generics, _) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));
    for id in 0..model.fields().len() {
        let ident = syn::Ident::new(&format!("__ST{}", id), proc_macro2::Span::call_site());
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
        use diesel::row::{Row, Field};
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
