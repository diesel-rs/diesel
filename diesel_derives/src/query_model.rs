use proc_macro2::TokenStream;
use syn::punctuated::Punctuated;
use syn::{parse_quote, parse_quote_spanned, DeriveInput};

use crate::model::Model;

pub(crate) fn derive(item: DeriveInput) -> syn::Result<TokenStream> {
    // other required traits
    let mut checks = Punctuated::new();
    if cfg!(feature = "postgres") {
        checks.push(parse_quote! {diesel::pg::Pg});
    }
    if cfg!(feature = "sqlite") {
        checks.push(parse_quote! {diesel::sqlite::Sqlite});
    }
    if cfg!(feature = "mysql") {
        checks.push(parse_quote! {diesel::mysql::Mysql});
    }
    let selectable =
        super::selectable::derive(item.clone(), (!checks.is_empty()).then_some(checks))?;
    let queryable = super::queryable::derive(item.clone())?;

    let ident = &item.ident;
    let model = Model::from_item(&item, false, false)?;
    let (original_impl_generics, ty_generics, original_where_clause) =
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
        let embed_fields = model
            .fields()
            .iter()
            .filter(|f| f.embed())
            .collect::<Vec<_>>();

        let table_name = &model.table_names()[0];
        let mut query_type =
            parse_quote!(<#table_name::table as diesel::query_builder::AsQuery>::Query);
        let mut query_expr = parse_quote!(#table_name::table);

        for embedded_field in embed_fields {
            let (join_fn, join_tpe) = if crate::util::is_option_ty(&embedded_field.ty) {
                (quote::quote! {left_join}, quote::quote! { LeftJoin })
            } else {
                (quote::quote!(inner_join), quote::quote! { InnerJoin })
            };
            let ty = crate::util::inner_of_option_ty(&embedded_field.ty);
            let span = embedded_field.span;
            query_expr = parse_quote_spanned! {span=>
                #query_expr.#join_fn(<#ty as diesel::prelude::QueryModel<__DB>>::base_query())
            };
            query_type = parse_quote_spanned! {span=>
                diesel::dsl::#join_tpe<#query_type, <#ty as diesel::prelude::QueryModel<__DB>>::BaseQuery>
            };
        }
        query_expr = parse_quote! {
            diesel::query_builder::AsQuery::as_query(#query_expr)
        };
        (query_expr, query_type)
    };

    let mut query_model = crate::util::wrap_in_dummy_mod(quote::quote! {
        impl #impl_generics QueryModel<__DB> for #ident #ty_generics #where_clause {
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
