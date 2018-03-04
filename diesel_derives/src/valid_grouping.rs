use quote;
use syn;

use model::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<quote::Tokens, Diagnostic> {
    let model = Model::from_item(&item)?;
    let struct_name = item.ident;

    let (_, ty_generics, _) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    let is_aggregate: syn::Type = if !model.fields().is_empty() {
        generics.params.push(parse_quote!(__IsAggregate));
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        for field in model.fields() {
            let field_ty = &field.ty;
            where_clause.predicates.push(parse_quote!(#field_ty: ValidGrouping<__GroupByClause, IsAggregate = __IsAggregate>));
        }
        parse_quote!(__IsAggregate)
    } else {
        parse_quote!(self::diesel::expression::NotAggregate)
    };

    generics.params.push(parse_quote!(__GroupByClause));
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let dummy_mod = format!("_impl_valid_grouping_for_{}", item.ident.as_ref().to_lowercase());
    Ok(wrap_in_dummy_mod(
        dummy_mod.into(),
        quote! {
            use self::diesel::expression::ValidGrouping;

            #[allow(non_camel_case_types)]
            impl #impl_generics ValidGrouping<__GroupByClause> for #struct_name #ty_generics
            #where_clause
            {
                type IsAggregate = #is_aggregate;
            }
        },
    ))
}
