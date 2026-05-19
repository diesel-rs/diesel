use std::collections::{HashMap, HashSet};

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{DeriveInput, Expr, Path, Result, Type, parse_quote};

use crate::attrs::AttributeSpanWrapper;
use crate::field::Field;
use crate::model::Model;
use crate::util::{inner_of_option_ty, is_option_ty, wrap_in_dummy_mod};

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, false, false)?;

    let struct_name = &item.ident;
    let table_name = &model.table_names()[0];

    let fields_for_update = model
        .fields()
        .iter()
        .filter(|f| {
            !model
                .primary_key_names
                .iter()
                .any(|p| f.column_name().map(|f| f == *p).unwrap_or_default())
        })
        .collect::<Vec<_>>();

    if fields_for_update.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::mixed_site(),
            "deriving `AsChangeset` on a structure that only contains primary keys isn't supported.\n\
             help: if you want to change the primary key of a row, you should do so with `.set(table::id.eq(new_id))`.\n\
             note: `#[derive(AsChangeset)]` never changes the primary key of a row.",
        ));
    }

    let treat_none_as_null = model.treat_none_as_null();

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let mut generate_borrowed_changeset = true;

    let mut direct_field_ty = Vec::with_capacity(fields_for_update.len());
    let mut direct_field_assign = Vec::with_capacity(fields_for_update.len());
    let mut ref_field_ty = Vec::with_capacity(fields_for_update.len());
    let mut ref_field_assign = Vec::with_capacity(fields_for_update.len());

    // Explicit trait bounds to improve error messages
    let mut field_ty_bounds = Vec::with_capacity(model.fields().len());
    let mut borrowed_field_ty_bounds = Vec::with_capacity(model.fields().len());
    let mut field_ty_bounds_guard = HashMap::new();
    let mut borrowed_field_ty_bounds_guard = HashMap::new();

    for field in fields_for_update {
        // skip this field while generating the update
        if field.skip_update() {
            continue;
        }
        // Use field-level attr. with fallback to the struct-level one.
        let treat_none_as_null = match &field.treat_none_as_null {
            Some(attr) => {
                if let Some(embed) = &field.embed {
                    return Err(syn::Error::new(
                        embed.attribute_span,
                        "`embed` and `treat_none_as_default_value` are mutually exclusive",
                    ));
                }

                if !is_option_ty(&field.ty) {
                    return Err(syn::Error::new(
                        field.ty.span(),
                        "expected `treat_none_as_null` field to be of type `Option<_>`",
                    ));
                }

                attr.item
            }
            None => treat_none_as_null,
        };

        match (field.serialize_as.as_ref(), field.embed()) {
            (Some(AttributeSpanWrapper { item: ty, .. }), false) => {
                direct_field_ty.push(field_changeset_ty_serialize_as(
                    field,
                    table_name,
                    ty,
                    treat_none_as_null,
                )?);
                direct_field_assign.push(field_changeset_expr_serialize_as(
                    field,
                    table_name,
                    ty,
                    treat_none_as_null,
                )?);
                field_ty_bounds.push(generate_field_bound(
                    field,
                    table_name,
                    Some(ty),
                    None,
                    &mut field_ty_bounds_guard,
                    treat_none_as_null,
                )?);

                generate_borrowed_changeset = false; // as soon as we hit one field with #[diesel(serialize_as)] there is no point in generating the impl of AsChangeset for borrowed structs
            }
            (Some(AttributeSpanWrapper { attribute_span, .. }), true) => {
                return Err(syn::Error::new(
                    *attribute_span,
                    "`#[diesel(embed)]` cannot be combined with `#[diesel(serialize_as)]`",
                ));
            }
            (None, true) => {
                direct_field_ty.push(field_changeset_ty_embed(field, None));
                direct_field_assign.push(field_changeset_expr_embed(field, None));
                ref_field_ty.push(field_changeset_ty_embed(field, Some(quote!(&'update))));
                ref_field_assign.push(field_changeset_expr_embed(field, Some(quote!(&))));
            }
            (None, false) => {
                direct_field_ty.push(field_changeset_ty(
                    field,
                    table_name,
                    None,
                    treat_none_as_null,
                )?);
                direct_field_assign.push(field_changeset_expr(
                    field,
                    table_name,
                    None,
                    treat_none_as_null,
                )?);
                ref_field_ty.push(field_changeset_ty(
                    field,
                    table_name,
                    Some(quote!(&'update)),
                    treat_none_as_null,
                )?);
                ref_field_assign.push(field_changeset_expr(
                    field,
                    table_name,
                    Some(quote!(&)),
                    treat_none_as_null,
                )?);

                field_ty_bounds.push(generate_field_bound(
                    field,
                    table_name,
                    None,
                    None,
                    &mut field_ty_bounds_guard,
                    treat_none_as_null,
                )?);

                borrowed_field_ty_bounds.push(generate_field_bound(
                    field,
                    table_name,
                    None,
                    Some(parse_quote!('update)),
                    &mut borrowed_field_ty_bounds_guard,
                    treat_none_as_null,
                )?);
            }
        }
    }

    let field_ty_bounds = field_ty_bounds
        .into_iter()
        .filter_map(|(type_to_check, bound)| {
            super::insertable::filter_bounds(&field_ty_bounds_guard, type_to_check, bound)
        });

    let changeset_owned = quote! {
        fn _check_owned #impl_generics ()
        where #(#field_ty_bounds,)*
        {}

        impl #impl_generics diesel::query_builder::AsChangeset for #struct_name #ty_generics
        #where_clause
        {
            type Target = #table_name::table;
            type Changeset = <(#(#direct_field_ty,)*) as diesel::query_builder::AsChangeset>::Changeset;

            fn as_changeset(self) -> <Self as diesel::query_builder::AsChangeset>::Changeset {
                diesel::query_builder::AsChangeset::as_changeset((#(#direct_field_assign,)*))
            }
        }
    };

    let changeset_borrowed = if generate_borrowed_changeset {
        let mut impl_generics = item.generics.clone();
        impl_generics.params.push(parse_quote!('update));
        let (impl_generics, _, _) = impl_generics.split_for_impl();
        let borrowed_field_ty_bounds =
            borrowed_field_ty_bounds
                .into_iter()
                .filter_map(|(type_to_check, bound)| {
                    super::insertable::filter_bounds(
                        &borrowed_field_ty_bounds_guard,
                        type_to_check,
                        bound,
                    )
                });
        let borrowed_check_params = item.generics.params.iter().map(|p| match p {
            syn::GenericParam::Lifetime(lp) => {
                let lt = &lp.lifetime;
                let bounds = lp.bounds.iter();
                if lp.bounds.is_empty() {
                    quote!(#lt: 'update)
                } else {
                    quote!(#lt: 'update + #(#bounds +)*)
                }
            }
            syn::GenericParam::Type(_) | syn::GenericParam::Const(_) => quote!(#p),
        });
        quote! {
            #[allow(clippy::multiple_bound_locations)]
            fn _check_borrowed<'update, #(#borrowed_check_params,)*>()
            where
                #(#borrowed_field_ty_bounds,)*
            {}

            impl #impl_generics diesel::query_builder::AsChangeset for &'update #struct_name #ty_generics
            where
            #where_clause
            (#(#ref_field_ty,)*): diesel::query_builder::AsChangeset,
            {
                type Target = #table_name::table;
                type Changeset = <(#(#ref_field_ty,)*) as diesel::query_builder::AsChangeset>::Changeset;

                fn as_changeset(self) -> <Self as diesel::query_builder::AsChangeset>::Changeset {
                    diesel::query_builder::AsChangeset::as_changeset((#(#ref_field_assign,)*))
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(wrap_in_dummy_mod(quote!(
        #changeset_owned

        #changeset_borrowed
    )))
}

fn generate_field_bound(
    field: &Field,
    table_name: &Path,
    ty: Option<&Type>,
    borrowed: Option<syn::Lifetime>,
    guard: &mut HashMap<Type, HashSet<Vec<syn::Lifetime>>>,
    treat_none_as_null: bool,
) -> Result<(Type, TokenStream)> {
    let (ty_for_guard, as_expression_bound) = super::insertable::generate_field_bound(
        field,
        table_name,
        ty.unwrap_or_else(|| field_changeset_actual_ty(field, treat_none_as_null)),
        treat_none_as_null,
        borrowed.clone(),
        guard,
    )?;
    Ok((ty_for_guard, as_expression_bound))
}

fn field_changeset_actual_ty(field: &Field, treat_none_as_null: bool) -> &Type {
    if !treat_none_as_null && is_option_ty(&field.ty) {
        inner_of_option_ty(&field.ty)
    } else {
        &field.ty
    }
}

fn field_changeset_ty_embed(field: &Field, lifetime: Option<TokenStream>) -> TokenStream {
    let field_ty = &field.ty;
    let span = Span::mixed_site().located_at(field.span);
    quote_spanned!(span=> #lifetime #field_ty)
}

fn field_changeset_expr_embed(field: &Field, lifetime: Option<TokenStream>) -> TokenStream {
    let field_name = &field.name;
    quote!(#lifetime self.#field_name)
}

fn field_changeset_ty(
    field: &Field,
    table_name: &Path,
    lifetime: Option<TokenStream>,
    treat_none_as_null: bool,
) -> Result<TokenStream> {
    let column_name = field.column_name()?.to_ident()?;
    if !treat_none_as_null && is_option_ty(&field.ty) {
        let field_ty = inner_of_option_ty(&field.ty);
        Ok(
            quote!(std::option::Option<diesel::dsl::Eq<#table_name::#column_name, #lifetime #field_ty>>),
        )
    } else {
        let field_ty = &field.ty;
        Ok(quote!(diesel::dsl::Eq<#table_name::#column_name, #lifetime #field_ty>))
    }
}

fn field_changeset_expr(
    field: &Field,
    table_name: &Path,
    lifetime: Option<TokenStream>,
    treat_none_as_null: bool,
) -> Result<TokenStream> {
    let field_name = &field.name;
    let column_name = field.column_name()?.to_ident()?;
    if !treat_none_as_null && is_option_ty(&field.ty) {
        if lifetime.is_some() {
            Ok(
                quote!(self.#field_name.as_ref().map(|x| diesel::ExpressionMethods::eq(#table_name::#column_name, x))),
            )
        } else {
            Ok(
                quote!(self.#field_name.map(|x| diesel::ExpressionMethods::eq(#table_name::#column_name, x))),
            )
        }
    } else {
        Ok(
            quote!(diesel::ExpressionMethods::eq(#table_name::#column_name, #lifetime self.#field_name)),
        )
    }
}

fn field_changeset_ty_serialize_as(
    field: &Field,
    table_name: &Path,
    ty: &Type,
    treat_none_as_null: bool,
) -> Result<TokenStream> {
    let column_name = field.column_name()?.to_ident()?;
    if !treat_none_as_null && is_option_ty(&field.ty) {
        let inner_ty = inner_of_option_ty(ty);
        Ok(quote!(std::option::Option<diesel::dsl::Eq<#table_name::#column_name, #inner_ty>>))
    } else {
        Ok(quote!(diesel::dsl::Eq<#table_name::#column_name, #ty>))
    }
}

fn field_changeset_expr_serialize_as(
    field: &Field,
    table_name: &Path,
    ty: &Type,
    treat_none_as_null: bool,
) -> Result<TokenStream> {
    let field_name = &field.name;
    let column_name = field.column_name()?.to_ident()?;
    let column: Expr = parse_quote!(#table_name::#column_name);
    if !treat_none_as_null && is_option_ty(&field.ty) {
        Ok(
            quote!(self.#field_name.map(|x| diesel::ExpressionMethods::eq(#column, ::std::convert::Into::<#ty>::into(x)))),
        )
    } else {
        Ok(quote!(#column.eq(::std::convert::Into::<#ty>::into(self.#field_name))))
    }
}
