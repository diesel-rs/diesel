use quote::Tokens;
use syn;

use util::*;

pub fn derive(item: syn::DeriveInput) -> Tokens {
    let struct_ty = if flag_present(&item.attrs, "foreign_derive") {
        match item.body {
            syn::Body::Struct(ref body) => body.fields()[0].ty.clone(),
            _ => panic!("foreign_derive cannot be used on enums"),
        }
    } else {
        struct_ty(item.ident.clone(), &item.generics)
    };

    let item_name = item.ident.as_ref().to_uppercase();
    let generics = syn::aster::from_generics(item.generics)
        .ty_param_id("__ST")
        .ty_param_id("__DB")
        .build();

    wrap_item_in_const(
        format!("_IMPL_FROM_SQL_ROW_FOR_{}", item_name).into(),
        quote!(
            impl#generics diesel::types::FromSqlRow<__ST, __DB> for #struct_ty
            where
                __DB: diesel::backend::Backend + diesel::types::HasSqlType<__ST>,
                Self: diesel::types::FromSql<__ST, __DB>,
            {
                fn build_from_row<R: diesel::row::Row<__DB>>(row: &mut R)
                    -> Result<Self, Box<::std::error::Error + Send + Sync>>
                {
                    diesel::types::FromSql::<__ST, __DB>::from_sql(row.take())
                }
            }

            impl#generics diesel::query_source::Queryable<__ST, __DB> for #struct_ty
            where
                __DB: diesel::backend::Backend + diesel::types::HasSqlType<__ST>,
                Self: diesel::types::FromSqlRow<__ST, __DB>,
            {
                type Row = Self;

                fn build(row: Self::Row) -> Self {
                    row
                }
            }
        ),
    )
}
