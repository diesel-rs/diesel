use proc_macro2::TokenStream;
use syn::DeriveInput;

use model::Model;
use util::{ty_for_foreign_derive, wrap_in_dummy_mod};

pub fn derive(item: DeriveInput) -> TokenStream {
    let model = Model::from_item(&item, true);

    if model.sql_types.is_empty() {
        abort_call_site!(
            "At least one `sql_type` is needed for deriving `AsExpression` on a structure."
        );
    }

    let struct_ty = ty_for_foreign_derive(&item, &model);

    let (impl_generics, ..) = item.generics.split_for_impl();
    let lifetimes = item.generics.lifetimes().collect::<Vec<_>>();
    let ty_params = item.generics.type_params().collect::<Vec<_>>();
    let const_params = item.generics.const_params().collect::<Vec<_>>();

    let tokens = model.sql_types.iter().map(|sql_type| {
        let lifetimes = &lifetimes;
        let ty_params = &ty_params;
        let const_params = &const_params;

        let tokens = quote!(
            impl<'expr, #(#lifetimes,)* #(#ty_params,)* #(#const_params,)*> AsExpression<#sql_type>
                for &'expr #struct_ty
            {
                type Expression = Bound<#sql_type, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'expr, #(#lifetimes,)* #(#ty_params,)* #(#const_params,)*> AsExpression<Nullable<#sql_type>>
                for &'expr #struct_ty
            {
                type Expression = Bound<Nullable<#sql_type>, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'expr2, 'expr, #(#lifetimes,)* #(#ty_params,)* #(#const_params,)*> AsExpression<#sql_type>
                for &'expr2 &'expr #struct_ty
            {
                type Expression = Bound<#sql_type, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<'expr2, 'expr, #(#lifetimes,)* #(#ty_params,)* #(#const_params,)*> AsExpression<Nullable<#sql_type>>
                for &'expr2 &'expr #struct_ty
            {
                type Expression = Bound<Nullable<#sql_type>, Self>;

                fn as_expression(self) -> Self::Expression {
                    Bound::new(self)
                }
            }

            impl<#(#lifetimes,)* #(#ty_params,)* __DB,  #(#const_params,)*> diesel::serialize::ToSql<Nullable<#sql_type>, __DB>
                for #struct_ty
            where
                __DB: diesel::backend::Backend,
                Self: ToSql<#sql_type, __DB>,
            {
                fn to_sql<'__b>(&'__b self, out: &mut Output<'__b, '_, __DB>) -> serialize::Result
                {
                    ToSql::<#sql_type, __DB>::to_sql(self, out)
                }
            }
        );

        if model.not_sized {
            tokens
        } else {
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
        }
    });

    wrap_in_dummy_mod(quote! {
        use diesel::expression::AsExpression;
        use diesel::internal::derives::as_expression::Bound;
        use diesel::sql_types::Nullable;
        use diesel::serialize::{self, ToSql, Output};

        #(#tokens)*
    })
}
