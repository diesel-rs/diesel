use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::DeriveInput;
use syn::Result;

use crate::model::Model;
use crate::util::{ty_for_foreign_derive, wrap_in_dummy_mod};

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, true, false)?;

    if model.sql_types.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::mixed_site(),
            "at least one `sql_type` is needed for deriving `AsExpression` on a structure.",
        ));
    }

    let struct_ty = ty_for_foreign_derive(&item, &model)?;

    // type generics are already handled by `ty_for_foreign_derive`
    let (impl_generics_plain, _, where_clause_plain) = item.generics.split_for_impl();

    let mut generics = item.generics.clone();
    generics.params.push(parse_quote!('__expr));

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let mut generics2 = generics.clone();
    generics2.params.push(parse_quote!('__expr2));
    let (impl_generics2, _, where_clause2) = generics2.split_for_impl();

    let tokens = model.sql_types.iter().map(|sql_type| {

        let mut to_sql_generics = item.generics.clone();
        to_sql_generics.params.push(parse_quote!(__DB));
        to_sql_generics.make_where_clause().predicates.push(parse_quote!(__DB: diesel::backend::Backend));
        to_sql_generics.make_where_clause().predicates.push(parse_quote!(Self: diesel::serialize::ToSql<#sql_type, __DB>));
        let (to_sql_impl_generics, _, to_sql_where_clause) = to_sql_generics.split_for_impl();

        let tokens = quote!(
            impl #impl_generics diesel::expression::AsExpression<#sql_type>
                for &'__expr #struct_ty #where_clause
            {
                type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            impl #impl_generics diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>
                for &'__expr #struct_ty #where_clause
            {
                type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            impl #impl_generics2 diesel::expression::AsExpression<#sql_type>
                for &'__expr2 &'__expr #struct_ty #where_clause2
            {
                type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            impl #impl_generics2 diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>
                for &'__expr2 &'__expr #struct_ty #where_clause2
            {
                type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, Self>;

                fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                    diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            impl #to_sql_impl_generics diesel::serialize::ToSql<diesel::sql_types::Nullable<#sql_type>, __DB>
                for #struct_ty #to_sql_where_clause
            {
                fn to_sql<'__b>(&'__b self, out: &mut diesel::serialize::Output<'__b, '_, __DB>) -> diesel::serialize::Result
                {
                    diesel::serialize::ToSql::<#sql_type, __DB>::to_sql(self, out)
                }
            }
        );

        if model.not_sized {
            tokens
        } else {
            quote!(
                #tokens

                impl #impl_generics_plain diesel::expression::AsExpression<#sql_type> for #struct_ty #where_clause_plain {
                    type Expression = diesel::internal::derives::as_expression::Bound<#sql_type, Self>;

                    fn as_expression(self) -> <Self as diesel::expression::AsExpression<#sql_type>>::Expression {
                        diesel::internal::derives::as_expression::Bound::new(self)
                    }
                }

                impl #impl_generics_plain diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>> for #struct_ty
                #where_clause_plain
                {
                    type Expression = diesel::internal::derives::as_expression::Bound<diesel::sql_types::Nullable<#sql_type>, Self>;

                    fn as_expression(self) -> <Self as diesel::expression::AsExpression<diesel::sql_types::Nullable<#sql_type>>>::Expression {
                        diesel::internal::derives::as_expression::Bound::new(self)
                    }
                }
            )
        }
    });

    Ok(wrap_in_dummy_mod(quote! {
        #(#tokens)*
    }))
}
