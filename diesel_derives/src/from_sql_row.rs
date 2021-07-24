use proc_macro2::*;
use syn;

use meta::*;
use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<TokenStream, Diagnostic> {
    let flags =
        MetaItem::with_name(&item.attrs, "diesel").unwrap_or_else(|| MetaItem::empty("diesel"));
    let struct_ty = ty_for_foreign_derive(&item, &flags)?;

    // To prevent double issue with mutable borrow below while creating where clause
    let initial_item = item.clone();

    let lifetimes = initial_item.generics.lifetimes().collect::<Vec<_>>();
    let ty_params = initial_item.generics.type_params().collect::<Vec<_>>();
    let const_params = initial_item.generics.const_params().collect::<Vec<_>>();

    {
        let where_clause = item
            .generics
            .where_clause
            .get_or_insert(parse_quote!(where));
        where_clause
            .predicates
            .push(parse_quote!(__DB: diesel::backend::Backend));
        where_clause
            .predicates
            .push(parse_quote!(__ST: diesel::sql_types::SingleValue));
        where_clause
            .predicates
            .push(parse_quote!(Self: FromSql<__ST, __DB>));
    }
    let (_, _, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, FromSql, Queryable};

        // Need to put __ST and __DB after lifetimes but before const params
        impl<#(#lifetimes,)* __ST, __DB, #(#ty_params,)* #(#const_params,)*> Queryable<__ST, __DB> for #struct_ty
        #where_clause
        {
            type Row = Self;

            fn build(row: Self::Row) -> deserialize::Result<Self> {
                Ok(row)
            }
        }
    }))
}
