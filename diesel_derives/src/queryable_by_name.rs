use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, DeriveInput, Ident, LitStr, Result, Type};

use crate::attrs::AttributeSpanWrapper;
use crate::field::{Field, FieldName};
use crate::model::Model;
use crate::util::wrap_in_dummy_mod;

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, false, false)?;

    let struct_name = &item.ident;
    let fields = &model.fields().iter().map(get_ident).collect::<Vec<_>>();
    let field_names = model.fields().iter().map(|f| &f.name);

    let initial_field_expr = model
        .fields()
        .iter()
        .map(|f| {
            let field_ty = &f.ty;

            if f.embed() {
                Ok(quote!(<#field_ty as QueryableByName<__DB>>::build(row)?))
            } else {
                let deserialize_ty = f.ty_for_deserialize();
                let name = f.column_name()?;
                let name = LitStr::new(&name.to_string(), name.span());
                Ok(quote!(
                   {
                       let field = diesel::row::NamedRow::get(row, #name)?;
                       <#deserialize_ty as Into<#field_ty>>::into(field)
                   }
                ))
            }
        })
        .collect::<Result<Vec<_>>>()?;

    let (_, ty_generics, ..) = item.generics.split_for_impl();
    let mut generics = item.generics.clone();
    generics
        .params
        .push(parse_quote!(__DB: diesel::backend::Backend));

    for field in model.fields() {
        let where_clause = generics.where_clause.get_or_insert(parse_quote!(where));
        let field_ty = field.ty_for_deserialize();
        if field.embed() {
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: QueryableByName<__DB>));
        } else {
            let st = sql_type(field, &model)?;
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: diesel::deserialize::FromSql<#st, __DB>));
        }
    }

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(wrap_in_dummy_mod(quote! {
        use diesel::deserialize::{self, QueryableByName};
        use diesel::row::{NamedRow};
        use diesel::sql_types::Untyped;

        impl #impl_generics QueryableByName<__DB>
            for #struct_name #ty_generics
        #where_clause
        {
            fn build<'__a>(row: &impl NamedRow<'__a, __DB>) -> deserialize::Result<Self>
            {
                #(
                    let mut #fields = #initial_field_expr;
                )*
                deserialize::Result::Ok(Self {
                    #(
                        #field_names: #fields,
                    )*
                })
            }
        }
    }))
}

fn get_ident(field: &Field) -> Ident {
    match &field.name {
        FieldName::Named(n) => n.clone(),
        FieldName::Unnamed(i) => Ident::new(&format!("field_{}", i.index), i.span),
    }
}

fn sql_type(field: &Field, model: &Model) -> Result<Type> {
    let table_name = &model.table_names()[0];

    match field.sql_type {
        Some(AttributeSpanWrapper { item: ref st, .. }) => Ok(st.clone()),
        None => {
            let column_name = field.column_name()?;
            Ok(parse_quote!(diesel::dsl::SqlTypeOf<#table_name::#column_name>))
        }
    }
}
