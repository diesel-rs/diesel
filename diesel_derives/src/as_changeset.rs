use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use syn::{parse_quote, DeriveInput, Expr, Path, Result, Type};

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
            proc_macro2::Span::call_site(),
            "Deriving `AsChangeset` on a structure that only contains primary keys isn't supported.\n\
             help: If you want to change the primary key of a row, you should do so with `.set(table::id.eq(new_id))`.\n\
             note: `#[derive(AsChangeset)]` never changes the primary key of a row."
        ));
    }

    let treat_none_as_null = model.treat_none_as_null();

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let mut generate_borrowed_changeset = true;

    let mut direct_field_ty = Vec::with_capacity(fields_for_update.len());
    let mut direct_field_assign = Vec::with_capacity(fields_for_update.len());
    let mut ref_field_ty = Vec::with_capacity(fields_for_update.len());
    let mut ref_field_assign = Vec::with_capacity(fields_for_update.len());

    for field in fields_for_update {
        // Use field-level attr. with fallback to the struct-level one.
        let treat_none_as_null = match &field.treat_none_as_null {
            Some(attr) => {
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

        match field.serialize_as.as_ref() {
            Some(AttributeSpanWrapper { item: ty, .. }) => {
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

                generate_borrowed_changeset = false; // as soon as we hit one field with #[diesel(serialize_as)] there is no point in generating the impl of AsChangeset for borrowed structs
            }
            None => {
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
            }
        }
    }

    let changeset_owned = quote! {
        impl #impl_generics AsChangeset for #struct_name #ty_generics
        #where_clause
        {
            type Target = #table_name::table;
            type Changeset = <(#(#direct_field_ty,)*) as AsChangeset>::Changeset;

            fn as_changeset(self) -> Self::Changeset {
                (#(#direct_field_assign,)*).as_changeset()
            }
        }
    };

    let changeset_borrowed = if generate_borrowed_changeset {
        let mut impl_generics = item.generics.clone();
        impl_generics.params.push(parse_quote!('update));
        let (impl_generics, _, _) = impl_generics.split_for_impl();

        quote! {
            impl #impl_generics AsChangeset for &'update #struct_name #ty_generics
            #where_clause
            {
                type Target = #table_name::table;
                type Changeset = <(#(#ref_field_ty,)*) as AsChangeset>::Changeset;

                fn as_changeset(self) -> Self::Changeset {
                    (#(#ref_field_assign,)*).as_changeset()
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(wrap_in_dummy_mod(quote!(
        use diesel::query_builder::AsChangeset;
        use diesel::prelude::*;

        #changeset_owned

        #changeset_borrowed
    )))
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
            Ok(quote!(self.#field_name.as_ref().map(|x| #table_name::#column_name.eq(x))))
        } else {
            Ok(quote!(self.#field_name.map(|x| #table_name::#column_name.eq(x))))
        }
    } else {
        Ok(quote!(#table_name::#column_name.eq(#lifetime self.#field_name)))
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
        Ok(quote!(self.#field_name.map(|x| #column.eq(::std::convert::Into::<#ty>::into(x)))))
    } else {
        Ok(quote!(#column.eq(::std::convert::Into::<#ty>::into(self.#field_name))))
    }
}
