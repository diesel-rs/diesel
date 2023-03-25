use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Result;
use syn::{DeriveInput, Ident};

use crate::model::Model;
use crate::parsers::PostgresType;
use crate::util::wrap_in_dummy_mod;

pub fn derive(item: DeriveInput) -> Result<TokenStream> {
    let model = Model::from_item(&item, true, false)?;

    let struct_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let sqlite_tokens = sqlite_tokens(&item, &model);
    let mysql_tokens = mysql_tokens(&item, &model);
    let pg_tokens = pg_tokens(&item, &model);

    Ok(wrap_in_dummy_mod(quote! {
        impl #impl_generics diesel::sql_types::SqlType
            for #struct_name #ty_generics
        #where_clause
        {
            type IsNull = diesel::sql_types::is_nullable::NotNull;
        }

        impl #impl_generics diesel::sql_types::SingleValue
            for #struct_name #ty_generics
        #where_clause
        {
        }

        #sqlite_tokens
        #mysql_tokens
        #pg_tokens
    }))
}

fn sqlite_tokens(item: &DeriveInput, model: &Model) -> Option<TokenStream> {
    model
        .sqlite_type
        .as_ref()
        .map(|sqlite_type| Ident::new(&sqlite_type.name.value(), Span::call_site()))
        .and_then(|ty| {
            if cfg!(not(feature = "sqlite")) {
                return None;
            }

            let struct_name = &item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

            Some(quote! {
                impl #impl_generics diesel::sql_types::HasSqlType<#struct_name #ty_generics>
                    for diesel::sqlite::Sqlite
                #where_clause
                {
                    fn metadata(_: &mut ()) -> diesel::sqlite::SqliteType {
                        diesel::sqlite::SqliteType::#ty
                    }
                }
            })
        })
}

fn mysql_tokens(item: &DeriveInput, model: &Model) -> Option<TokenStream> {
    model
        .mysql_type
        .as_ref()
        .map(|mysql_type| Ident::new(&mysql_type.name.value(), Span::call_site()))
        .and_then(|ty| {
            if cfg!(not(feature = "mysql")) {
                return None;
            }

            let struct_name = &item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

            Some(quote! {
                impl #impl_generics diesel::sql_types::HasSqlType<#struct_name #ty_generics>
                    for diesel::mysql::Mysql
                #where_clause
                {
                    fn metadata(_: &mut ()) -> diesel::mysql::MysqlType {
                        diesel::mysql::MysqlType::#ty
                    }
                }
            })
        })
}

fn pg_tokens(item: &DeriveInput, model: &Model) -> Option<TokenStream> {
    model.postgres_type.as_ref().and_then(|ty| {
        if cfg!(not(feature = "postgres")) {
            return None;
        }

        let struct_name = &item.ident;
        let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

        let metadata_fn = match ty {
            PostgresType::Fixed(oid, array_oid) => quote!(
                fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
                    PgTypeMetadata::new(#oid, #array_oid)
                }
            ),
            PostgresType::Lookup(type_name, Some(type_schema)) => quote!(
                fn metadata(lookup: &mut Self::MetadataLookup) -> PgTypeMetadata {
                    lookup.lookup_type(#type_name, Some(#type_schema))
                }
            ),
            PostgresType::Lookup(type_name, None) => quote!(
                fn metadata(lookup: &mut Self::MetadataLookup) -> PgTypeMetadata {
                    lookup.lookup_type(#type_name, None)
                }
            ),
        };

        Some(quote! {
            use diesel::pg::{PgMetadataLookup, PgTypeMetadata};

            impl #impl_generics diesel::sql_types::HasSqlType<#struct_name #ty_generics>
                for diesel::pg::Pg
            #where_clause
            {
                #metadata_fn
            }
        })
    })
}
