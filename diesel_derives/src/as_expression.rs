use proc_macro2;
use syn;

use meta::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let flags =
        MetaItem::with_name(&item.attrs, "diesel").unwrap_or_else(|| MetaItem::empty("diesel"));
    let is_sized = !flags.has_flag("not_sized");

    let sql_types = MetaItem::all_with_name(&item.attrs, "sql_type");
    let any_sql_types = !sql_types.is_empty();
    let sql_types = sql_types
        .into_iter()
        .filter_map(|attr| attr.ty_value().map_err(Diagnostic::emit).ok());

    let (impl_generics, ..) = item.generics.split_for_impl();
    let lifetimes = item.generics.lifetimes().collect::<Vec<_>>();
    let ty_params = item.generics.type_params().collect::<Vec<_>>();
    let struct_ty = ty_for_foreign_derive(&item, &flags)?;

    let tokens = sql_types.map(|sql_type| {
        let lifetimes = &lifetimes;
        let ty_params = &ty_params;
        let tokens = quote!(
            impl<'expr, #(#lifetimes,)* #(#ty_params,)*> AsExpression<#sql_type>
                for &'expr #struct_ty
            {
                type Expression = Bound<#sql_type, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'expr, #(#lifetimes,)* #(#ty_params,)*> AsExpression<Nullable<#sql_type>>
                for &'expr #struct_ty
            {
                type Expression = Bound<Nullable<#sql_type>, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'expr2, 'expr, #(#lifetimes,)* #(#ty_params,)*> AsExpression<#sql_type>
                for &'expr2 &'expr #struct_ty
            {
                type Expression = Bound<#sql_type, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'expr2, 'expr, #(#lifetimes,)* #(#ty_params,)*> AsExpression<Nullable<#sql_type>>
                for &'expr2 &'expr #struct_ty
            {
                type Expression = Bound<Nullable<#sql_type>, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<#(#lifetimes,)* #(#ty_params,)* __DB> diesel::serialize::ToSql<Nullable<#sql_type>, __DB>
                for #struct_ty
            where
                __DB: diesel::backend::Backend,
                Self: ToSql<#sql_type, __DB>,
            {
                fn to_sql<W: std::io::Write>(&self, out: &mut Output<W, __DB>) -> serialize::Result {
                    ToSql::<#sql_type, __DB>::to_sql(self, out)
                }
            }
        );
        if is_sized {
            quote!(
                #tokens

                impl#impl_generics AsExpression<#sql_type> for #struct_ty {
                    type Expression = Bound<#sql_type, Self>;

                    fn as_expression(self) -> Self::Expression {
                        Bound::new(self)
                    }
                }

                impl#impl_generics AsExpression<Nullable<#sql_type>> for #struct_ty {
                    type Expression = Bound<Nullable<#sql_type>, Self>;

                    fn as_expression(self) -> Self::Expression {
                        Bound::new(self)
                    }
                }
            )
        } else {
            tokens
        }
    });

    if any_sql_types {
        Ok(wrap_in_dummy_mod(quote! {
            use diesel::expression::AsExpression;
            use diesel::expression::bound::Bound;
            use diesel::sql_types::Nullable;
            use diesel::serialize::{self, ToSql, Output};

            #(#tokens)*
        }))
    } else {
        Ok(quote!())
    }
}
