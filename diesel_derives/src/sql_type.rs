use proc_macro2;
use syn;

use meta::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let struct_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let sqlite_tokens = sqlite_tokens(&item);
    let mysql_tokens = mysql_tokens(&item);
    let pg_tokens = pg_tokens(&item);

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

fn sqlite_tokens(item: &syn::DeriveInput) -> Option<proc_macro2::TokenStream> {
    MetaItem::with_name(&item.attrs, "sqlite_type")
        .map(|attr| attr.expect_ident_value())
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

fn mysql_tokens(item: &syn::DeriveInput) -> Option<proc_macro2::TokenStream> {
    MetaItem::with_name(&item.attrs, "mysql_type")
        .map(|attr| attr.expect_ident_value())
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

fn pg_tokens(item: &syn::DeriveInput) -> Option<proc_macro2::TokenStream> {
    MetaItem::with_name(&item.attrs, "postgres")
        .map(|attr| {
            if let Some(x) = get_type_name(&attr)? {
                Ok(x)
            } else if let Some(x) = get_oids(&attr)? {
                Ok(x)
            } else {
                Err(attr
                    .span()
                    .error("Missing required options")
                    .help("Valid options are `type_name` or `oid` and `array_oid`"))
            }
        })
        .and_then(|res| res.map_err(Diagnostic::emit).ok())
        .and_then(|ty| {
            if cfg!(not(feature = "postgres")) {
                return None;
            }

            let struct_name = &item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

            let metadata_fn = match ty {
                PgType::Fixed { oid, array_oid } => quote!(
                    fn metadata(_: &mut Self::MetadataLookup) -> PgTypeMetadata {
                        PgTypeMetadata::new(#oid, #array_oid)
                    }
                ),
                PgType::Lookup(type_name, Some(type_schema)) => quote!(
                    fn metadata(lookup: &mut Self::MetadataLookup) -> PgTypeMetadata {
                        lookup.lookup_type(#type_name, Some(#type_schema))
                    }
                ),
                PgType::Lookup(type_name, None) => quote!(
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

fn get_type_name(attr: &MetaItem) -> Result<Option<PgType>, Diagnostic> {
    let schema = attr.nested_item("type_schema")?;
    Ok(attr.nested_item("type_name")?.map(|ty| {
        attr.warn_if_other_options(&["type_name", "type_schema"]);
        PgType::Lookup(
            ty.expect_str_value(),
            schema.map(|schema| schema.expect_str_value()),
        )
    }))
}

fn get_oids(attr: &MetaItem) -> Result<Option<PgType>, Diagnostic> {
    if let Some(oid) = attr.nested_item("oid")? {
        attr.warn_if_other_options(&["oid", "array_oid"]);
        let array_oid = attr.required_nested_item("array_oid")?.expect_int_value();
        let oid = oid.expect_int_value();
        Ok(Some(PgType::Fixed {
            oid: oid as u32,
            array_oid: array_oid as u32,
        }))
    } else {
        Ok(None)
    }
}

enum PgType {
    Fixed { oid: u32, array_oid: u32 },
    Lookup(String, Option<String>),
}
