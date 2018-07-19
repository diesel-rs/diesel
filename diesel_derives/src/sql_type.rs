use proc_macro2;
use proc_macro2::*;
use syn;

use meta::*;
use util::*;

pub fn derive(item: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Diagnostic> {
    let struct_name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let sqlite_tokens = sqlite_tokens(&item);
    let mysql_tokens = mysql_tokens(&item);
    let pg_tokens = pg_tokens(&item);

    let dummy_name = format!("_impl_sql_type_for_{}", item.ident);
    Ok(wrap_in_dummy_mod(
        Ident::new(&dummy_name.to_lowercase(), Span::call_site()),
        quote! {
            impl #impl_generics diesel::sql_types::NotNull
                for #struct_name #ty_generics
            #where_clause
            {
            }

            impl #impl_generics diesel::sql_types::SingleValue
                for #struct_name #ty_generics
            #where_clause
            {
            }

            #sqlite_tokens
            #mysql_tokens
            #pg_tokens
        },
    ))
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
                    fn metadata(_: &()) -> diesel::sqlite::SqliteType {
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
                    fn metadata(_: &()) -> diesel::mysql::MysqlType {
                        diesel::mysql::MysqlType::#ty
                    }
                }
            })
        })
}

fn pg_tokens(item: &syn::DeriveInput) -> Option<proc_macro2::TokenStream> {
    MetaItem::with_name(&item.attrs, "postgres")
        .and_then(|attr| {
            get_type_name(&attr)
                .or_else(|| get_oids(&attr))
                .or_else(|| {
                    attr.span()
                        .error("Missing required options")
                        .help("Valid options are `type_name` or `oid` and `array_oid`")
                        .emit();
                    None
                })
        })
        .and_then(|ty| {
            if cfg!(not(feature = "postgres")) {
                return None;
            }

            let struct_name = &item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

            let metadata_fn = match ty {
                PgType::Fixed { oid, array_oid } => quote!(
                    fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
                        PgTypeMetadata {
                            oid: #oid,
                            array_oid: #array_oid,
                        }
                    }
                ),
                PgType::Lookup(type_name) => quote!(
                    fn metadata(lookup: &PgMetadataLookup) -> PgTypeMetadata {
                        lookup.lookup_type(#type_name)
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

fn get_type_name(attr: &MetaItem) -> Option<PgType> {
    attr.nested_item("type_name").ok().map(|ty| {
        attr.warn_if_other_options(&["type_name"]);
        PgType::Lookup(ty.expect_str_value())
    })
}

fn get_oids(attr: &MetaItem) -> Option<PgType> {
    attr.nested_item("oid").ok().map(|oid| {
        attr.warn_if_other_options(&["oid", "array_oid"]);
        let array_oid = attr.nested_item("array_oid")
            .emit_error()
            .map(|a| a.expect_int_value())
            .unwrap_or(0);
        let oid = oid.expect_int_value();
        PgType::Fixed {
            oid: oid as u32,
            array_oid: array_oid as u32,
        }
    })
}

enum PgType {
    Fixed { oid: u32, array_oid: u32 },
    Lookup(String),
}
