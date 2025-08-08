use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::borrow::Cow;
use syn::spanned::Spanned;
use syn::{parse_quote, DeriveInput, Result};

use crate::field::Field;
use crate::model::{CheckForBackend, Model};
use crate::util::wrap_in_dummy_mod;

pub fn derive(
    item: DeriveInput,
    check_for_backend: Option<CheckForBackend>,
) -> Result<TokenStream> {
    let model = Model::from_item(&item, false, false)?;

    let (original_impl_generics, ty_generics, original_where_clause) =
        item.generics.split_for_impl();

    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    for embed_field in model.fields().iter().filter(|f| f.embed()) {
        let embed_ty = &embed_field.ty;
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#embed_ty: Selectable<__DB>));
    }

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let struct_name = &item.ident;

    let mut compile_errors: Vec<syn::Error> = Vec::new();
    let field_select_expression_type_builders = model
        .fields()
        .iter()
        .map(|f| field_select_expression_ty_builder(f, &model, &mut compile_errors))
        .collect::<Result<Vec<_>>>()?;
    let field_select_expression_types = field_select_expression_type_builders
        .iter()
        .map(|f| f.type_with_backend(&parse_quote!(__DB)))
        .collect::<Vec<_>>();
    let field_select_expressions = model
        .fields()
        .iter()
        .map(|f| field_column_inst(f, &model))
        .collect::<Result<Vec<_>>>()?;

    let check_function = if let Some(backends) = model
        .check_for_backend
        .as_ref()
        .or(check_for_backend.as_ref())
        .and_then(|c| match c {
            CheckForBackend::Backends(punctuated) => Some(punctuated),
            CheckForBackend::Disabled(_lit_bool) => None,
        }) {
        let field_check_bound = model
            .fields()
            .iter()
            .zip(&field_select_expression_type_builders)
            .flat_map(|(f, ty_builder)| {
                backends.iter().map(move |b| {
                    let span = Span::mixed_site().located_at(f.ty.span());
                    let field_ty = to_field_ty_bound(f.ty_for_deserialize())?;
                    let ty = ty_builder.type_with_backend(b);
                    Ok(syn::parse_quote_spanned! {span =>
                        #field_ty: diesel::deserialize::FromSqlRow<diesel::dsl::SqlTypeOf<#ty>, #b>
                    })
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let where_clause = &mut original_where_clause.cloned();
        let where_clause = where_clause.get_or_insert_with(|| parse_quote!(where));
        for field_check in field_check_bound {
            where_clause.predicates.push(field_check);
        }
        Some(quote::quote! {
            fn _check_field_compatibility #original_impl_generics()
                #where_clause
            {}
        })
    } else {
        None
    };

    let errors: TokenStream = compile_errors
        .into_iter()
        .map(|e| e.into_compile_error())
        .collect();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::expression::Selectable;

        impl #impl_generics Selectable<__DB>
            for #struct_name #ty_generics
        #where_clause
        {
            type SelectExpression = (#(#field_select_expression_types,)*);

            fn construct_selection() -> Self::SelectExpression {
                (#(#field_select_expressions,)*)
            }
        }

        #check_function

        #errors
    }))
}

fn to_field_ty_bound(field_ty: &syn::Type) -> Result<TokenStream> {
    match field_ty {
        syn::Type::Reference(r) => {
            use crate::quote::ToTokens;
            // references are not supported for checking for now
            //
            // (How ever you can even have references in a `Queryable` struct anyway)
            Err(syn::Error::new(
                field_ty.span(),
                format!(
                    "references are not supported in `Queryable` types\n\
                         consider using `std::borrow::Cow<'{}, {}>` instead",
                    r.lifetime
                        .as_ref()
                        .expect("It's a struct field so it must have a named lifetime")
                        .ident,
                    r.elem.to_token_stream()
                ),
            ))
        }
        field_ty => Ok(quote::quote! {
            #field_ty
        }),
    }
}

fn field_select_expression_ty_builder<'a>(
    field: &'a Field,
    model: &Model,
    compile_errors: &mut Vec<syn::Error>,
) -> Result<FieldSelectExpressionTyBuilder<'a>> {
    if let Some(ref select_expression) = field.select_expression {
        use dsl_auto_type::auto_type::expression_type_inference as type_inference;
        let expr = &select_expression.item;
        let (inferred_type, errors) = type_inference::infer_expression_type(
            expr,
            field.select_expression_type.as_ref().map(|t| &t.item),
            &type_inference::InferrerSettings::builder()
                .dsl_path(parse_quote!(diesel::dsl))
                .function_types_case(crate::AUTO_TYPE_DEFAULT_FUNCTION_TYPE_CASE)
                .method_types_case(crate::AUTO_TYPE_DEFAULT_METHOD_TYPE_CASE)
                .build(),
        );
        compile_errors.extend(errors);
        Ok(FieldSelectExpressionTyBuilder::Always(
            quote::quote!(#inferred_type),
        ))
    } else if let Some(ref select_expression_type) = field.select_expression_type {
        let ty = &select_expression_type.item;
        Ok(FieldSelectExpressionTyBuilder::Always(quote!(#ty)))
    } else if field.embed() {
        Ok(FieldSelectExpressionTyBuilder::EmbedSelectable {
            embed_ty: &field.ty,
        })
    } else {
        let table_name = &model.table_names()[0];
        let column_name = field.column_name()?.to_ident()?;
        let span = Span::call_site();
        Ok(FieldSelectExpressionTyBuilder::Always(
            quote_spanned!(span=> #table_name::#column_name),
        ))
    }
}

enum FieldSelectExpressionTyBuilder<'a> {
    Always(TokenStream),
    EmbedSelectable { embed_ty: &'a syn::Type },
}

impl FieldSelectExpressionTyBuilder<'_> {
    fn type_with_backend(&self, backend: &syn::TypePath) -> Cow<'_, TokenStream> {
        match self {
            FieldSelectExpressionTyBuilder::Always(ty) => Cow::Borrowed(ty),
            FieldSelectExpressionTyBuilder::EmbedSelectable { embed_ty } => {
                Cow::Owned(quote!(<#embed_ty as Selectable<#backend>>::SelectExpression))
            }
        }
    }
}

fn field_column_inst(field: &Field, model: &Model) -> Result<TokenStream> {
    if let Some(ref select_expression) = field.select_expression {
        let expr = &select_expression.item;
        Ok(quote!(#expr))
    } else if field.embed() {
        let embed_ty = &field.ty;
        Ok(quote!(<#embed_ty as Selectable<__DB>>::construct_selection()))
    } else {
        let table_name = &model.table_names()[0];
        let column_name = field.column_name()?.to_ident()?;
        let span = Span::call_site();
        Ok(quote_spanned!(span=> #table_name::#column_name))
    }
}
