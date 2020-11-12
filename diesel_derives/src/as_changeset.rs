use proc_macro2::TokenStream;
use syn::{DeriveInput, Path};

use field::Field;
use model::Model;
use util::{inner_of_option_ty, is_option_ty, wrap_in_dummy_mod};

pub fn derive(item: DeriveInput) -> TokenStream {
    let model = Model::from_item(&item, false);

    let struct_name = &item.ident;
    let table_name = model.table_name();

    let fields_for_update = model
        .fields()
        .iter()
        .filter(|f| !model.primary_key_names.contains(f.column_name()))
        .collect::<Vec<_>>();

    if fields_for_update.is_empty() {
        abort_call_site!(
            "Deriving `AsChangeset` on a structure that only contains primary keys isn't supported.";
            help = "If you want to change the primary key of a row, you should do so with `.set(table::id.eq(new_id))`.";
            note = "`#[derive(AsChangeset)]` never changes the primary key of a row.";
        )
    }

    let treat_none_as_null = model.treat_none_as_null();

    let (_, ty_generics, where_clause) = item.generics.split_for_impl();
    let mut impl_generics = item.generics.clone();
    impl_generics.params.push(parse_quote!('update));
    let (impl_generics, _, _) = impl_generics.split_for_impl();

    let ref_changeset_ty = fields_for_update.iter().map(|field| {
        field_changeset_ty(
            field,
            &table_name,
            treat_none_as_null,
            Some(quote!(&'update)),
        )
    });
    let ref_changeset_expr = fields_for_update
        .iter()
        .map(|field| field_changeset_expr(field, &table_name, treat_none_as_null, Some(quote!(&))));

    let direct_changeset_ty = fields_for_update
        .iter()
        .map(|field| field_changeset_ty(field, &table_name, treat_none_as_null, None));
    let direct_changeset_expr = fields_for_update
        .iter()
        .map(|field| field_changeset_expr(field, &table_name, treat_none_as_null, None));

    wrap_in_dummy_mod(quote!(
        use diesel::query_builder::AsChangeset;
        use diesel::prelude::*;

        impl #impl_generics AsChangeset for &'update #struct_name #ty_generics
        #where_clause
        {
            type Target = #table_name::table;
            type Changeset = <(#(#ref_changeset_ty,)*) as AsChangeset>::Changeset;

            fn as_changeset(self) -> Self::Changeset {
                (#(#ref_changeset_expr,)*).as_changeset()
            }
        }

        impl #impl_generics AsChangeset for #struct_name #ty_generics
        #where_clause
        {
            type Target = #table_name::table;
            type Changeset = <(#(#direct_changeset_ty,)*) as AsChangeset>::Changeset;

            fn as_changeset(self) -> Self::Changeset {
                (#(#direct_changeset_expr,)*).as_changeset()
            }
        }
    ))
}

fn field_changeset_ty(
    field: &Field,
    table_name: &Path,
    treat_none_as_null: bool,
    lifetime: Option<TokenStream>,
) -> TokenStream {
    let column_name = field.column_name();
    if !treat_none_as_null && is_option_ty(&field.ty) {
        let field_ty = inner_of_option_ty(&field.ty);
        quote!(std::option::Option<diesel::dsl::Eq<#table_name::#column_name, #lifetime #field_ty>>)
    } else {
        let field_ty = &field.ty;
        quote!(diesel::dsl::Eq<#table_name::#column_name, #lifetime #field_ty>)
    }
}

fn field_changeset_expr(
    field: &Field,
    table_name: &Path,
    treat_none_as_null: bool,
    lifetime: Option<TokenStream>,
) -> TokenStream {
    let field_name = &field.name;
    let column_name = field.column_name();
    if !treat_none_as_null && is_option_ty(&field.ty) {
        if lifetime.is_some() {
            quote!(self.#field_name.as_ref().map(|x| #table_name::#column_name.eq(x)))
        } else {
            quote!(self.#field_name.map(|x| #table_name::#column_name.eq(x)))
        }
    } else {
        quote!(#table_name::#column_name.eq(#lifetime self.#field_name))
    }
}
