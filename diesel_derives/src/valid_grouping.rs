use proc_macro2::TokenStream;
use syn::DeriveInput;

use model::Model;
use util::{ty_for_foreign_derive, wrap_in_dummy_mod};

pub fn derive(mut item: DeriveInput) -> TokenStream {
    let model = Model::from_item(&item, true);
    let struct_ty = ty_for_foreign_derive(&item, &model);

    if model.aggregate {
        // Make all type parameters valid groupings.
        for type_param in item.generics.type_params_mut() {
            type_param
                .bounds
                .push(parse_quote!(ValidGrouping<__GroupByClause>));
        }

        item.generics.params.push(parse_quote!(__GroupByClause));
        let (impl_generics, _, where_clause) = item.generics.split_for_impl();

        wrap_in_dummy_mod(quote! {
            use diesel::expression::{ValidGrouping, MixedAggregates, is_aggregate};

            impl #impl_generics ValidGrouping<__GroupByClause> for #struct_ty
            #where_clause
            {
                type IsAggregate = is_aggregate::Yes;
            }
        })
    } else {
        // We now build up:
        // - the `IsAggregate` associated type to use as this grouping's aggregate
        // - the where clause constraining type parameters to be valid groupings
        // - the where clause constraining type parameters as valid aggregates

        // If there are existing where clause predicates, we'll enrich this existing where clause.
        let has_predicates = item
            .generics
            .where_clause
            .as_ref()
            .map(|where_clause| !where_clause.predicates.is_empty())
            .unwrap_or(false);

        // If there are type params, we'll need to constrain them.
        let has_aggregates = item.generics.type_params().next().is_some();

        let (is_aggregate, where_clause) = if has_predicates || has_aggregates {
            let mut where_clause = if has_predicates {
                // There are existing predicates, we'll enrich the existing where clause.
                let where_clause = &item.generics.where_clause;
                quote!(#where_clause,).to_string()
            } else {
                // There are type params to aggregate and constrain in the where clause, but no
                // existing predicates: let's build a where clause from scratch.
                String::from("where ")
            };

            let mut is_aggregate = String::new();
            for (idx, type_param) in item.generics.type_params().enumerate() {
                // Make all type parameters valid groupings.
                where_clause.push_str(&format!(
                    "{}: ValidGrouping<__GroupByClause>,",
                    quote!(#type_param)
                ));

                let aggregate = quote!(#type_param::IsAggregate);

                // This `ValidGrouping`'s associated type `IsAggregate` is always the last type
                // parameter's:
                // - when there's only parameter: we simply use its `IsAggregate`
                // - when there's multiple: we'll chain the type parameters pair-wise with
                //   `MixedAggregates`, and the result will be the last `Output`
                is_aggregate = if idx == 0 {
                    aggregate.to_string()
                } else {
                    // Build-up the `MixedAggregates` chain both:
                    // 1) in the where clause
                    where_clause.push_str(&format!(
                        "{left}: MixedAggregates<{right}>,",
                        left = is_aggregate,
                        right = aggregate,
                    ));

                    // 2) for the final aggregate
                    format!(
                        "<{left} as MixedAggregates <{right}>>::Output",
                        left = is_aggregate,
                        right = aggregate,
                    )
                };
            }

            // Only parse the complex associated type once.
            let is_aggregate = is_aggregate
                .parse()
                .expect("Unexpected lexing error while parsing `is_aggregate`");

            // Only parse the complex where clause once.
            let where_clause: TokenStream = where_clause
                .parse()
                .expect("Unexpected lexing error while parsing `where_clause`");

            (is_aggregate, Some(where_clause))
        } else {
            (quote!(is_aggregate::Never), None)
        };

        item.generics.params.push(parse_quote!(__GroupByClause));
        let (impl_generics, _, _) = item.generics.split_for_impl();

        // Inline expansion of util::wrap_in_dummy_mod, to avoid quoting/parsing a big module.
        quote! {
            #[allow(unused_imports)]
            const _: () = {
                // This import is not actually redundant. When using diesel_derives
                // inside of diesel, `diesel` doesn't exist as an extern crate, and
                // to work around that it contains a private
                // `mod diesel { pub use super::*; }` that this import will then
                // refer to. In all other cases, this imports refers to the extern
                // crate diesel.
                use diesel;
                use diesel::expression::{ValidGrouping, MixedAggregates, is_aggregate};

                impl #impl_generics ValidGrouping<__GroupByClause> for #struct_ty
                #where_clause
                {
                    type IsAggregate = #is_aggregate;
                }
            };
        }
    }
}
