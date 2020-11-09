use proc_macro2::*;
use syn;

use meta::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<TokenStream, Diagnostic> {
    let flags =
        MetaItem::with_name(&item.attrs, "diesel").unwrap_or_else(|| MetaItem::empty("diesel"));
    let struct_ty = ty_for_foreign_derive(&item, &flags)?;

    let (impl_generics, _, where_clause) = item.generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, Queryable};

        impl #impl_generics Queryable for #struct_ty
        #where_clause
        {
            type Row = Self;

            fn build(row: Self::Row) -> deserialize::Result<Self> {
                Ok(row)
            }
        }
    }))
}
