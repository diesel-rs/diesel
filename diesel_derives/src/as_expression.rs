use quote::Tokens;
use syn;

use util::*;

pub fn derive(item: syn::DeriveInput) -> Tokens {
    let item_name = item.ident.as_ref().to_uppercase();
    let is_sized = !flag_present(&item.attrs, "not_sized");
    let sql_types = item.attrs
        .iter()
        .filter(|attr| attr.name() == "sql_type")
        .map(|attr| ty_value_of_attr(attr, "sql_type"));

    let struct_ty = if flag_present(&item.attrs, "foreign_derive") {
        match item.body {
            syn::Body::Struct(ref body) => body.fields()[0].ty.clone(),
            _ => panic!("foreign_derive cannot be used on enums"),
        }
    } else {
        struct_ty(item.ident.clone(), &item.generics)
    };

    let generics = &item.generics;
    let syn::Generics {
        ref lifetimes,
        ref ty_params,
        ..
    } = *generics;

    let tokens = sql_types.map(|sql_type| {
        let tokens = quote!(
            impl<'expr, #(#lifetimes,)* #(#ty_params,)*> diesel::expression::AsExpression<#sql_type>
                for &'expr #struct_ty
            {
                type Expression = diesel::expression::bound::Bound<#sql_type, Self>;

                fn as_expression(self) -> Self::Expression {
                    diesel::expression::bound::Bound::new(self)
                }
            }

            impl<'expr, #(#lifetimes,)* #(#ty_params,)*> diesel::expression::AsExpression<diesel::types::Nullable<#sql_type>>
                for &'expr #struct_ty
            {
                type Expression = diesel::expression::bound::Bound<diesel::types::Nullable<#sql_type>, Self>;

                fn as_expression(self) -> Self::Expression {
                    diesel::expression::bound::Bound::new(self)
                }
            }

            impl<'expr2, 'expr, #(#lifetimes,)* #(#ty_params,)*> diesel::expression::AsExpression<#sql_type>
                for &'expr2 &'expr #struct_ty
            {
                type Expression = diesel::expression::bound::Bound<#sql_type, Self>;

                fn as_expression(self) -> Self::Expression {
                    diesel::expression::bound::Bound::new(self)
                }
            }

            impl<'expr2, 'expr, #(#lifetimes,)* #(#ty_params,)*> diesel::expression::AsExpression<diesel::types::Nullable<#sql_type>>
                for &'expr2 &'expr #struct_ty
            {
                type Expression = diesel::expression::bound::Bound<diesel::types::Nullable<#sql_type>, Self>;

                fn as_expression(self) -> Self::Expression {
                    diesel::expression::bound::Bound::new(self)
                }
            }

            impl<#(#lifetimes,)* #(#ty_params,)* __DB> diesel::types::ToSql<diesel::types::Nullable<#sql_type>, __DB>
                for #struct_ty
            where
                __DB: diesel::backend::Backend + diesel::types::HasSqlType<#sql_type>,
                Self: diesel::types::ToSql<#sql_type, __DB>,
            {
                fn to_sql<W: ::std::io::Write>(&self, out: &mut diesel::types::ToSqlOutput<W, __DB>) -> ::std::result::Result<diesel::types::IsNull, Box<::std::error::Error + Send + Sync>> {
                    diesel::types::ToSql::<#sql_type, __DB>::to_sql(self, out)
                }
            }
        );
        if is_sized {
            quote!(
                #tokens

                impl#generics diesel::expression::AsExpression<#sql_type> for #struct_ty {
                    type Expression = diesel::expression::bound::Bound<#sql_type, Self>;

                    fn as_expression(self) -> Self::Expression {
                        diesel::expression::bound::Bound::new(self)
                    }
                }

                impl#generics diesel::expression::AsExpression<diesel::types::Nullable<#sql_type>> for #struct_ty {
                    type Expression = diesel::expression::bound::Bound<diesel::types::Nullable<#sql_type>, Self>;

                    fn as_expression(self) -> Self::Expression {
                        diesel::expression::bound::Bound::new(self)
                    }
                }
            )
        } else {
            tokens
        }
    });

    wrap_item_in_const(
        format!("_IMPL_AS_EXPRESSION_FOR_{}", item_name).into(),
        quote!(#(#tokens)*),
    )
}
