use proc_macro2::*;
use syn;

use meta::*;
use util::*;

pub fn derive(mut item: syn::DeriveInput) -> Result<TokenStream, Diagnostic> {
    let flags =
        MetaItem::with_name(&item.attrs, "diesel").unwrap_or_else(|| MetaItem::empty("diesel"));
    let struct_ty = ty_for_foreign_derive(&item, &flags)?;
    let type_params = item
        .generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect::<Vec<_>>();
    for type_param in type_params {
        let where_clause = item.generics.make_where_clause();
        where_clause
            .predicates
            .push(parse_quote!(#type_param: ValidGrouping<__GroupByClause>));
    }

    let is_aggregate = flags.has_flag("aggregate");

    if is_aggregate {
        item.generics.params.push(parse_quote!(__GroupByClause));
        let (impl_generics, _, where_clause) = item.generics.split_for_impl();
        Ok(wrap_in_dummy_mod(quote! {
            use diesel::expression::{ValidGrouping, MixedAggregates, is_aggregate};

            impl #impl_generics ValidGrouping<__GroupByClause> for #struct_ty
            #where_clause
            {
                type IsAggregate = is_aggregate::Yes;
            }
        }))
    } else {
        let mut aggregates = item
            .generics
            .type_params()
            .map(|t| parse_quote!(#t::IsAggregate))
            .collect::<Vec<syn::Type>>()
            .into_iter();
        let is_aggregate = aggregates
            .next()
            .map(|first| {
                let where_clause = item.generics.make_where_clause();
                aggregates.fold(first, |left, right| {
                    where_clause.predicates.push(parse_quote!(
                        #left: MixedAggregates<#right>
                    ));
                    parse_quote!(<#left as MixedAggregates<#right>>::Output)
                })
            })
            .unwrap_or_else(|| parse_quote!(is_aggregate::Never));
        item.generics.params.push(parse_quote!(__GroupByClause));
        let (impl_generics, _, where_clause) = item.generics.split_for_impl();

        Ok(wrap_in_dummy_mod(quote! {
            use diesel::expression::{ValidGrouping, MixedAggregates, is_aggregate};

            impl #impl_generics ValidGrouping<__GroupByClause> for #struct_ty
            #where_clause
            {
                type IsAggregate = #is_aggregate;
            }
        }))
    }
}
