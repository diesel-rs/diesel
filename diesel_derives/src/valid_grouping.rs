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

    let type_params = item
        .generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect::<Vec<_>>();

    for type_param in type_params {
        let where_clause = item.generics.make_where_clause();
        where_clause
            .predicates
            .push(parse_quote!(#type_param: diesel::expression::ValidGrouping<__GroupByClause>));
    }

    if model.aggregate {
        item.generics.params.push(parse_quote!(__GroupByClause));
        let (impl_generics, _, where_clause) = item.generics.split_for_impl();

        Ok(wrap_in_dummy_mod(quote! {
            impl #impl_generics diesel::expression::ValidGrouping<__GroupByClause> for #struct_ty
            #where_clause
            {
                type IsAggregate = diesel::expression::is_aggregate::Yes;
            }
        }))
    } else {
        let mut aggregates = item
            .generics
            .type_params()
            .map(
                |t| quote!(<#t as diesel::expression::ValidGrouping<__GroupByClause>>::IsAggregate),
            )
            .collect::<Vec<_>>()
            .into_iter();

        let is_aggregate = aggregates
            .next()
            .map(|first| {
                let where_clause = item.generics.make_where_clause();
                aggregates.fold(first, |left, right| {
                    where_clause.predicates.push(parse_quote!(
                        #left: diesel::expression::MixedAggregates<#right>
                    ));
                    quote!(<#left as diesel::expression::MixedAggregates<#right>>::Output)
                })
            })
            .unwrap_or_else(|| quote!(diesel::expression::is_aggregate::Never));

        item.generics.params.push(parse_quote!(__GroupByClause));
        let (impl_generics, _, where_clause) = item.generics.split_for_impl();

        Ok(wrap_in_dummy_mod(quote! {

            impl #impl_generics diesel::expression::ValidGrouping<__GroupByClause> for #struct_ty
            #where_clause
            {
                type IsAggregate = #is_aggregate;
            }
        }))
    }
}
