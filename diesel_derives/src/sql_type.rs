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
                    fn metadata(_: &()) -> diesel::mysql::MysqlTypeMetadata {
                        diesel::mysql::MysqlTypeMetadata {
                            data_type: diesel::mysql::MysqlType::#ty,
                            is_unsigned: false,
                        }
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

            let (metadata_fn, const_metadata) = match ty {
                PgType::Fixed { oid, array_oid } => {
                    let metadata_fn = quote!(
                        fn metadata(_: &PgMetadataLookup) -> PgTypeMetadata {
                            PgTypeMetadata {
                                oid: #oid,
                                array_oid: #array_oid,
                            }
                        }
                    );

                    let const_metadata = quote! {
                        impl #impl_generics diesel::pg::StaticSqlType for #struct_name #ty_generics
                        {
                            const OID: std::num::NonZeroU32 = unsafe {std::num::NonZeroU32::new_unchecked(#oid) };
                            const ARRAY_OID: std::num::NonZeroU32 = unsafe {std::num::NonZeroU32::new_unchecked(#array_oid) };
                        }
                    };

                    (metadata_fn, Some(const_metadata))
                }
                PgType::Lookup(type_name) => (quote!(
                    fn metadata(lookup: &PgMetadataLookup) -> PgTypeMetadata {
                        lookup.lookup_type(#type_name)
                    }
                ), None)
            };

            Some(quote! {
                use diesel::pg::{PgMetadataLookup, PgTypeMetadata};

                impl #impl_generics diesel::sql_types::HasSqlType<#struct_name #ty_generics>
                    for diesel::pg::Pg
                #where_clause
                {
                    #metadata_fn
                }

                #const_metadata
            })
        })
}

fn get_type_name(attr: &MetaItem) -> Result<Option<PgType>, Diagnostic> {
    Ok(attr.nested_item("type_name")?.map(|ty| {
        attr.warn_if_other_options(&["type_name"]);
        PgType::Lookup(ty.expect_str_value())
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
    Lookup(String),
}
