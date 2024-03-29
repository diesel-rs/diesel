use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::DeriveInput;
use syn::Result;

use crate::model::Model;
use crate::util::{ty_for_foreign_derive, wrap_in_dummy_mod};

pub fn derive(mut item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, true, false)?;
    let struct_ty = ty_for_foreign_derive(&item, &model)?;

    {
        item.generics.params.push(parse_quote!(__DB));
        item.generics.params.push(parse_quote!(__ST));
        let where_clause = item.generics.make_where_clause();
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
    let (impl_generics, _, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, FromSql, Queryable};

        // Need to put __ST and __DB after lifetimes but before const params
        impl #impl_generics Queryable<__ST, __DB> for #struct_ty
        #where_clause
        {
            type Row = Self;

            fn build(row: Self::Row) -> deserialize::Result<Self> {
                Ok(row)
            }
        }
    }))
}
