use crate::attrs::AttributeSpanWrapper;
use crate::field::Field;
use crate::model::Model;
use crate::util::{inner_of_option_ty, is_option_ty, wrap_in_dummy_mod};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use quote::quote_spanned;
use std::collections::{HashMap, HashSet};
use syn::spanned::Spanned as _;
use syn::{DeriveInput, Expr, Path, Result, Type};
use syn::{Lifetime, parse_quote};

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, false, true)?;

    let tokens = model
        .table_names()
        .iter()
        .map(|table_name| derive_into_single_table(&item, &model, table_name))
        .collect::<Result<Vec<_>>>()?;

    Ok(wrap_in_dummy_mod(quote! {
        #(#tokens)*
    }))
}

fn derive_into_single_table(
    item: &DeriveInput,
    model: &Model,
    table_name: &Path,
) -> Result<TokenStream> {
    let treat_none_as_default_value = model.treat_none_as_default_value();
    let struct_name = &item.ident;

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let mut generate_borrowed_insert = true;

    let mut direct_field_ty = Vec::with_capacity(model.fields().len());
    let mut direct_field_assign = Vec::with_capacity(model.fields().len());
    let mut ref_field_ty = Vec::with_capacity(model.fields().len());
    let mut ref_field_assign = Vec::with_capacity(model.fields().len());

    // Explicit trait bounds to improve error messages
    let mut field_ty_bounds = Vec::with_capacity(model.fields().len());
    let mut borrowed_field_ty_bounds = Vec::with_capacity(model.fields().len());
    let mut field_ty_bounds_guard = HashMap::new();
    let mut borrowed_field_ty_bounds_guard = HashMap::new();

    for field in model.fields() {
        // skip this field while generating the insertion
        if field.skip_insertion() {
            continue;
        }
        // Use field-level attr. with fallback to the struct-level one.
        let treat_none_as_default_value = match &field.treat_none_as_default_value {
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
                        "expected `treat_none_as_default_value` field to be of type `Option<_>`",
                    ));
                }

                attr.item
            }
            None => treat_none_as_default_value,
        };

        match (field.serialize_as.as_ref(), field.embed()) {
            (None, true) => {
                direct_field_ty.push(field_ty_embed(field, None));
                direct_field_assign.push(field_expr_embed(field, None));
                ref_field_ty.push(field_ty_embed(field, Some(quote!(&'insert))));
                ref_field_assign.push(field_expr_embed(field, Some(quote!(&))));
            }
            (None, false) => {
                direct_field_ty.push(field_ty(
                    field,
                    table_name,
                    None,
                    treat_none_as_default_value,
                )?);
                direct_field_assign.push(field_expr(
                    field,
                    table_name,
                    None,
                    treat_none_as_default_value,
                )?);
                ref_field_ty.push(field_ty(
                    field,
                    table_name,
                    Some(quote!(&'insert)),
                    treat_none_as_default_value,
                )?);
                ref_field_assign.push(field_expr(
                    field,
                    table_name,
                    Some(quote!(&)),
                    treat_none_as_default_value,
                )?);

                field_ty_bounds.push(generate_field_bound(
                    field,
                    table_name,
                    &field.ty,
                    treat_none_as_default_value,
                    false,
                    &mut field_ty_bounds_guard,
                )?);

                borrowed_field_ty_bounds.push(generate_field_bound(
                    field,
                    table_name,
                    &field.ty,
                    treat_none_as_default_value,
                    true,
                    &mut borrowed_field_ty_bounds_guard,
                )?);
            }
            (Some(AttributeSpanWrapper { item: ty, .. }), false) => {
                direct_field_ty.push(field_ty_serialize_as(
                    field,
                    table_name,
                    ty,
                    treat_none_as_default_value,
                )?);
                direct_field_assign.push(field_expr_serialize_as(
                    field,
                    table_name,
                    ty,
                    treat_none_as_default_value,
                )?);

                field_ty_bounds.push(generate_field_bound(
                    field,
                    table_name,
                    ty,
                    treat_none_as_default_value,
                    false,
                    &mut field_ty_bounds_guard,
                )?);

                generate_borrowed_insert = false; // as soon as we hit one field with #[diesel(serialize_as)] there is no point in generating the impl of Insertable for borrowed structs
            }
            (Some(AttributeSpanWrapper { attribute_span, .. }), true) => {
                return Err(syn::Error::new(
                    *attribute_span,
                    "`#[diesel(embed)]` cannot be combined with `#[diesel(serialize_as)]`",
                ));
            }
        }
    }

    let field_ty_bounds = field_ty_bounds
        .into_iter()
        .filter_map(|(type_to_check, bound)| {
            filter_bounds(&field_ty_bounds_guard, type_to_check, bound)
        });
    let insert_owned = quote! {
        impl #impl_generics diesel::insertable::Insertable<#table_name::table> for #struct_name #ty_generics
        where
            #where_clause
            #(#field_ty_bounds,)*
        {
            type Values = <(#(#direct_field_ty,)*) as diesel::insertable::Insertable<#table_name::table>>::Values;

            fn values(self) -> <(#(#direct_field_ty,)*) as diesel::insertable::Insertable<#table_name::table>>::Values {
                diesel::insertable::Insertable::<#table_name::table>::values((#(#direct_field_assign,)*))
            }
        }
    };

    let insert_borrowed = if generate_borrowed_insert {
        let mut impl_generics = item.generics.clone();
        impl_generics.params.push(parse_quote!('insert));
        let (impl_generics, ..) = impl_generics.split_for_impl();
        let borrowed_field_ty_bounds =
            borrowed_field_ty_bounds
                .into_iter()
                .filter_map(|(type_to_check, bound)| {
                    filter_bounds(&borrowed_field_ty_bounds_guard, type_to_check, bound)
                });

        quote! {
            impl #impl_generics diesel::insertable::Insertable<#table_name::table>
                for &'insert #struct_name #ty_generics
            where
                #where_clause
                #(#borrowed_field_ty_bounds,)*
            {
                type Values = <(#(#ref_field_ty,)*) as diesel::insertable::Insertable<#table_name::table>>::Values;

                fn values(self) -> <(#(#ref_field_ty,)*) as diesel::insertable::Insertable<#table_name::table>>::Values {
                    diesel::insertable::Insertable::<#table_name::table>::values((#(#ref_field_assign,)*))
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #insert_owned

        #insert_borrowed

        impl #impl_generics diesel::internal::derives::insertable::UndecoratedInsertRecord<#table_name::table>
                for #struct_name #ty_generics
            #where_clause
        {
        }
    })
}

// this function exists to filter out bounds that are essentially the same but appear with different lifetimes.
//
// That's something that is not supported by rustc currently: https://github.com/rust-lang/rust/issues/21974
// It might be fixed with the new trait solver which might land 2026
fn filter_bounds(
    guard: &HashMap<Type, HashSet<Vec<Lifetime>>>,
    type_to_check: syn::Type,
    bound: TokenStream,
) -> Option<TokenStream> {
    let count = guard
        .get(&type_to_check)
        .map(|t| t.len())
        .unwrap_or_default();
    (count <= 1).then_some(bound)
}

fn field_ty_embed(field: &Field, lifetime: Option<TokenStream>) -> TokenStream {
    let field_ty = &field.ty;
    let span = Span::mixed_site().located_at(field.span);
    quote_spanned!(span=> #lifetime #field_ty)
}

fn field_expr_embed(field: &Field, lifetime: Option<TokenStream>) -> TokenStream {
    let field_name = &field.name;
    quote!(#lifetime self.#field_name)
}

fn field_ty_serialize_as(
    field: &Field,
    table_name: &Path,
    ty: &Type,
    treat_none_as_default_value: bool,
) -> Result<TokenStream> {
    let column_name = field.column_name()?.to_ident()?;
    let span = Span::mixed_site().located_at(field.span);
    if treat_none_as_default_value {
        let inner_ty = inner_of_option_ty(ty);

        Ok(quote_spanned! {span=>
            std::option::Option<diesel::dsl::Eq<
                #table_name::#column_name,
                #inner_ty,
            >>
        })
    } else {
        Ok(quote_spanned! {span=>
            diesel::dsl::Eq<
                #table_name::#column_name,
                #ty,
            >
        })
    }
}

fn field_expr_serialize_as(
    field: &Field,
    table_name: &Path,
    ty: &Type,
    treat_none_as_default_value: bool,
) -> Result<TokenStream> {
    let field_name = &field.name;
    let column_name = field.column_name()?.to_ident()?;
    let column = quote!(#table_name::#column_name);
    if treat_none_as_default_value {
        if is_option_ty(ty) {
            Ok(
                quote!(::std::convert::Into::<#ty>::into(self.#field_name).map(|v| diesel::ExpressionMethods::eq(#column, v))),
            )
        } else {
            Ok(
                quote!(std::option::Option::Some(diesel::ExpressionMethods::eq(#column, ::std::convert::Into::<#ty>::into(self.#field_name)))),
            )
        }
    } else {
        Ok(
            quote!(diesel::ExpressionMethods::eq(#column, ::std::convert::Into::<#ty>::into(self.#field_name))),
        )
    }
}

fn field_ty(
    field: &Field,
    table_name: &Path,
    lifetime: Option<TokenStream>,
    treat_none_as_default_value: bool,
) -> Result<TokenStream> {
    let column_name = field.column_name()?.to_ident()?;
    let span = Span::mixed_site().located_at(field.span);
    if treat_none_as_default_value {
        let inner_ty = inner_of_option_ty(&field.ty);

        Ok(quote_spanned! {span=>
            std::option::Option<diesel::dsl::Eq<
                #table_name::#column_name,
                #lifetime #inner_ty,
            >>
        })
    } else {
        let inner_ty = &field.ty;

        Ok(quote_spanned! {span=>
            diesel::dsl::Eq<
                #table_name::#column_name,
                #lifetime #inner_ty,
            >
        })
    }
}

fn field_expr(
    field: &Field,
    table_name: &Path,
    lifetime: Option<TokenStream>,
    treat_none_as_default_value: bool,
) -> Result<TokenStream> {
    let field_name = &field.name;
    let column_name = field.column_name()?.to_ident()?;

    let column: Expr = parse_quote!(#table_name::#column_name);
    if treat_none_as_default_value {
        if is_option_ty(&field.ty) {
            if lifetime.is_some() {
                Ok(
                    quote!(self.#field_name.as_ref().map(|x| diesel::ExpressionMethods::eq(#column, x))),
                )
            } else {
                Ok(quote!(self.#field_name.map(|x| diesel::ExpressionMethods::eq(#column, x))))
            }
        } else {
            Ok(
                quote!(std::option::Option::Some(diesel::ExpressionMethods::eq(#column, #lifetime self.#field_name))),
            )
        }
    } else {
        Ok(quote!(diesel::ExpressionMethods::eq(#column, #lifetime self.#field_name)))
    }
}

/// Generate explicit trait bound with field span to improve error messages
fn generate_field_bound(
    field: &Field,
    table_name: &Path,
    ty: &Type,
    treat_none_as_default_value: bool,
    borrowed: bool,
    guard: &mut HashMap<Type, HashSet<Vec<Lifetime>>>,
) -> Result<(syn::Type, TokenStream)> {
    let column_name = field.column_name()?.to_ident()?;
    let span = Span::mixed_site().located_at(field.span);
    let ty_to_check = if treat_none_as_default_value {
        inner_of_option_ty(ty)
    } else {
        ty
    };
    let mut type_for_guard = ty_to_check.clone();
    // we use syn::visit_mut here to:
    // * Collect all lifetimes that appear in a certain type
    // * Normalize all lifetimes appearing in a certain type to 'static to be able
    // to use the type as key for a hashmap to find out if the type already appeared in
    // another bound. The value in the hashmap then contains all the different lifetime
    // variants, which lets us reason about whether there are different variants or not.
    let mut collector = LifetimeCollector::default();
    syn::visit_mut::visit_type_mut(&mut collector, &mut type_for_guard);
    let life_times = collector.lifetimes;

    guard
        .entry(type_for_guard.clone())
        .or_default()
        .insert(life_times);
    let bound_ty = if borrowed {
        quote_spanned! {span=> &'insert #ty_to_check}
    } else {
        quote_spanned! {span=> #ty_to_check}
    };
    let bound = quote_spanned! {span=>
        #bound_ty: diesel::expression::AsExpression<
            <#table_name::#column_name as diesel::Expression>::SqlType
        >
    };
    Ok((type_for_guard, bound))
}

#[derive(Default)]
struct LifetimeCollector {
    lifetimes: Vec<Lifetime>,
}

impl syn::visit_mut::VisitMut for LifetimeCollector {
    fn visit_lifetime_mut(&mut self, i: &mut syn::Lifetime) {
        self.lifetimes
            .push(std::mem::replace(i, syn::parse_quote!('static)));

        syn::visit_mut::visit_lifetime_mut(self, i);
    }
}
