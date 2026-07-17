use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;
use syn::{DeriveInput, parse_quote};

use crate::model::Model;
use crate::selectable::FieldSelectExpressionTyBuilder;

fn generate_default_checks(
    model: &Model,
    original_impl_generics: &syn::ImplGenerics<'_>,
    original_where_clause: Option<&syn::WhereClause>,
    field_select_expression_type_builders: &[FieldSelectExpressionTyBuilder<'_>],
) -> syn::Result<TokenStream> {
    let checks = [
        (
            quote::quote! {
                diesel::internal::derives::has_query::expand_pg!
            },
            parse_quote! {diesel::pg::Pg},
            parse_quote! {_check_field_compatibility_pg},
        ),
        (
            quote::quote! {
                diesel::internal::derives::has_query::expand_sqlite!
            },
            parse_quote! {diesel::sqlite::Sqlite},
            parse_quote! {_check_field_compatibility_sqlite},
        ),
        (
            quote::quote! {
                diesel::internal::derives::has_query::expand_mysql!
            },
            parse_quote! {diesel::mysql::Mysql},
            parse_quote! {_check_field_compatibility_mysql},
        ),
    ]
    .into_iter()
    .map(|(backend_macro, backend_ty, function)| {
        let mut backend = Punctuated::new();
        backend.push(backend_ty);

        let check = super::selectable::generate_check_function(
            model,
            original_impl_generics,
            original_where_clause,
            field_select_expression_type_builders,
            &backend,
            function,
        )?;
        Ok(quote::quote! {
            #backend_macro {#check}
        })
    })
    .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote::quote! { #(#checks)*})
}

pub(crate) fn derive(item: DeriveInput) -> syn::Result<TokenStream> {
    // other required traits

    let selectable = super::selectable::derive(item.clone(), Some(generate_default_checks))?;
    let queryable = super::queryable::derive(item.clone())?;

    let ident = &item.ident;
    let model = Model::from_item(&item, false, false)?;
    let (_original_impl_generics, ty_generics, _original_where_clause) =
        item.generics.split_for_impl();

    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let mut errors = Vec::new();

    let (query_expr, query_type) = if let Some(base_query) = model.base_query {
        if let Some(query_type) = model.base_query_type {
            (base_query, query_type)
        } else {
            use dsl_auto_type::auto_type::expression_type_inference as type_inference;

            let (inferred_type, infer_errors) = type_inference::infer_expression_type(
                &base_query,
                None,
                &type_inference::InferrerSettings::builder()
                    .dsl_path(parse_quote!(diesel::dsl))
                    .function_types_case(crate::AUTO_TYPE_DEFAULT_FUNCTION_TYPE_CASE)
                    .method_types_case(crate::AUTO_TYPE_DEFAULT_METHOD_TYPE_CASE)
                    .build(),
            );

            errors = infer_errors
                .into_iter()
                .map(|e| e.into_compile_error())
                .collect();
            (base_query, inferred_type)
        }
    } else {
        let table_name = &model.table_names()[0];
        let query_type =
            parse_quote!(<#table_name::table as diesel::query_builder::AsQuery>::Query);

        let query_expr = parse_quote! {
            diesel::query_builder::AsQuery::as_query(#table_name::table)
        };
        (query_expr, query_type)
    };

    let mut query_model = crate::util::wrap_in_dummy_mod(quote::quote! {
        impl #impl_generics diesel::HasQuery<__DB> for #ident #ty_generics #where_clause {
            type BaseQuery = #query_type;

            fn base_query() -> Self::BaseQuery {
                #query_expr
            }

        }
        #(#errors)*
    });
    query_model.extend(selectable);
    query_model.extend(queryable);
    Ok(query_model)
}
