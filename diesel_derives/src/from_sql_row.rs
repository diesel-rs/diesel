use proc_macro2::*;
use syn;

use meta::*;
use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<TokenStream, Diagnostic> {
    let flags =
        MetaItem::with_name(&item.attrs, "diesel").unwrap_or_else(|| MetaItem::empty("diesel"));
    let struct_ty = ty_for_foreign_derive(&item, &flags)?;

    item.generics.params.push(parse_quote!(__ST));
    item.generics.params.push(parse_quote!(__DB));
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
    let (impl_generics, _, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, FromSql, Queryable};

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
