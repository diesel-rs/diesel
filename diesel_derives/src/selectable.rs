use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::DeriveInput;
use syn::{parse_quote, Result};

use crate::field::Field;
use crate::model::Model;
use crate::util::wrap_in_dummy_mod;

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, false, false)?;

    let (_, ty_generics, original_where_clause) = item.generics.split_for_impl();

    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    for embed_field in model.fields().iter().filter(|f| f.embed()) {
        let embed_ty = &embed_field.ty;
        generics
            .where_clause
            .get_or_insert_with(|| parse_quote!(where))
            .predicates
            .push(parse_quote!(#embed_ty: Selectable<__DB>));
    }

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let struct_name = &item.ident;

    let mut compile_errors: Vec<syn::Error> = Vec::new();
    let field_columns_ty = model
        .fields()
        .iter()
        .map(|f| field_column_ty(f, &model, &mut compile_errors))
        .collect::<Result<Vec<_>>>()?;
    let field_columns_inst = model
        .fields()
        .iter()
        .map(|f| field_column_inst(f, &model))
        .collect::<Result<Vec<_>>>()?;

    let check_function = if let Some(ref backends) = model.check_for_backend {
        let field_check_bound = model
            .fields()
            .iter()
            .zip(&field_columns_ty)
            .filter(|(f, _)| !f.embed())
            .flat_map(|(f, ty)| {
                backends.iter().map(move |b| {
                    let span = f.ty.span();
                    let field_ty = to_field_ty_bound(f.ty_for_deserialize())?;
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
            fn _check_field_compatibility #impl_generics()
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
            type SelectExpression = (#(#field_columns_ty,)*);

            fn construct_selection() -> Self::SelectExpression {
                (#(#field_columns_inst,)*)
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
                    "References are not supported in `Queryable` types\n\
                         Consider using `std::borrow::Cow<'{}, {}>` instead",
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

fn field_column_ty(
    field: &Field,
    model: &Model,
    compile_errors: &mut Vec<syn::Error>,
) -> Result<TokenStream> {
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
        Ok(quote::quote!(#inferred_type))
    } else if let Some(ref select_expression_type) = field.select_expression_type {
        let ty = &select_expression_type.item;
        Ok(quote!(#ty))
    } else if field.embed() {
        let embed_ty = &field.ty;
        Ok(quote!(<#embed_ty as Selectable<__DB>>::SelectExpression))
    } else {
        let table_name = &model.table_names()[0];
        let column_name = field.column_name()?.to_ident()?;
        Ok(quote!(#table_name::#column_name))
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
        Ok(quote!(#table_name::#column_name))
    }
}
