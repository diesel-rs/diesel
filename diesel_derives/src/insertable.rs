use syn;
use quote;
use util::wrap_item_in_const;

use model::Model;

pub fn derive_insertable(item: syn::DeriveInput) -> quote::Tokens {
    let model = t!(Model::from_item(&item, "Insertable"));

    if !model.has_table_name_annotation() {
        return quote!(
            compile_error!(
                r#"`#[derive(Insertable)]` requires the struct to be annotated \
                with `#[table_name="something"]`"#
            );
        );
    }

    if !model.generics.ty_params.is_empty() {
        return quote!(compile_error!("`#[derive(Insertable)]` does not support generic types"););
    }

    let struct_name = &model.name;
    let struct_ty = &model.ty;
    let table_name = &model.table_name();
    let dummy_const_name = model.dummy_const_name("INSERTABLE");
    let syn::Generics {
        lifetimes,
        ty_params,
        ..
    } = model.generics;

    let mut lifetimes_with_insert = lifetimes.clone();
    lifetimes_with_insert.push(syn::LifetimeDef::new("'insert"));

    let ty_params = &ty_params;

    let values_types = model
        .attrs
        .iter()
        .map(|a| attr_to_value_type(a, table_name))
        .collect::<Vec<_>>();
    let values = model
        .attrs
        .iter()
        .map(|a| attr_to_insertable_values(a, table_name))
        .collect::<Vec<_>>();

    let derived_insertable = if model.attrs.is_empty() {
        let error_message = format!(
            "Failed to derive `Insertable` for `{}`: `Insertable` \
             cannot be used on structs with empty fields",
            struct_name
        );
        quote!(compile_error!(#error_message);)
    } else {
        quote! {
            impl<#(#lifetimes_with_insert,)* #(#ty_params,)*> diesel::insertable::Insertable<#table_name::table>
                for &'insert #struct_ty
            {
                type Values = <(#(#values_types,)*) as diesel::insertable::Insertable<#table_name::table>>::Values;

                fn values(self) -> Self::Values {
                    diesel::insertable::Insertable::values((#(#values,)*))
                }
            }
            impl<#(#lifetimes,)* #(#ty_params,)*> diesel::query_builder::UndecoratedInsertRecord<#table_name::table>
                for #struct_ty
            {
            }
        }
    };
    wrap_item_in_const(dummy_const_name, derived_insertable)
}

fn attr_to_insertable_values(a: &::attr::Attr, table_name: &syn::Ident) -> quote::Tokens {
    let column_name = a.column_name();
    let field_name = a.field_name();
    let column = quote!(#table_name::#column_name);
    if ::util::is_option_ty(&a.ty) {
        quote!{
            self.#field_name.as_ref()
                .and_then(|v| Some(diesel::ExpressionMethods::eq(#column, v)))
        }
    } else {
        quote! {
            Some(diesel::ExpressionMethods::eq(#column, &self.#field_name))
        }
    }
}

fn attr_to_value_type(a: &::attr::Attr, table_name: &syn::Ident) -> quote::Tokens {
    let field_name = a.column_name();
    let mut field_ty = &a.ty;
    if let Some(ty) = ::util::inner_of_option_ty(field_ty) {
        field_ty = ty;
    }
    quote! {
        Option<diesel::dsl::Eq<#table_name::#field_name, &'insert #field_ty>>
    }
}
