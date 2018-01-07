use quote::Tokens;
use syn;

use util::*;

pub fn derive(item: syn::DeriveInput) -> Tokens {
    let struct_ty = struct_ty(item.ident.clone(), &item.generics);
    let item_name = item.ident.as_ref().to_uppercase();
    let generics = &item.generics;
    let pg_tokens = pg_tokens(&item.attrs, generics, &struct_ty);
    let sqlite_tokens = sqlite_tokens(&item.attrs, generics, &struct_ty);
    let mysql_tokens = mysql_tokens(&item.attrs, generics, &struct_ty);

    wrap_item_in_const(
        format!("_IMPL_SQL_TYPE_FOR_{}", item_name).into(),
        quote!(
            impl #generics diesel::types::NotNull for #struct_ty {}
            impl #generics diesel::types::SingleValue for #struct_ty {}
            #pg_tokens
            #sqlite_tokens
            #mysql_tokens
        ),
    )
}

fn pg_tokens(
    attrs: &[syn::Attribute],
    generics: &syn::Generics,
    struct_ty: &syn::Ty,
) -> Option<Tokens> {
    use syn::{Lit, MetaItem, NestedMetaItem};

    fn error() -> ! {
        panic!(
            "#[postgres] must be in the form \
             #[postgres(oid = \"1\", array_oid = \"2\")] \
             or #[postgres(type_name = \"my_type\")]"
        );
    }

    if cfg!(not(feature = "postgres")) {
        return None;
    }

    attr_with_name(attrs, "postgres").map(|attr| {
        let items = match attr.value {
            MetaItem::List(_, ref items) => items,
            _ => error(),
        };
        let items = items
            .iter()
            .filter_map(|item| match *item {
                NestedMetaItem::MetaItem(ref item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();

        let str_value = |name| {
            items
                .iter()
                .find(|a| a.name() == name)
                .map(|item| match **item {
                    MetaItem::NameValue(_, Lit::Str(ref s, _)) => s,
                    _ => error(),
                })
        };

        let oid = str_value("oid").map(|s| s.parse::<u32>().expect("Invalid number"));
        let array_oid = str_value("array_oid").map(|s| s.parse::<u32>().expect("Invalid number"));
        let type_name = str_value("type_name");

        match (oid, array_oid, type_name) {
            (Some(oid), Some(array_oid), None) => quote!(
                impl #generics diesel::types::HasSqlType<#struct_ty> for diesel::pg::Pg {
                    fn metadata(_: &diesel::pg::PgMetadataLookup) -> diesel::pg::PgTypeMetadata {
                        diesel::pg::PgTypeMetadata {
                            oid: #oid,
                            array_oid: #array_oid,
                        }
                    }
                }
            ),
            (None, None, Some(type_name)) => quote!(
                impl #generics diesel::types::HasSqlType<#struct_ty> for diesel::pg::Pg {
                    fn metadata(lookup: &diesel::pg::PgMetadataLookup) -> diesel::pg::PgTypeMetadata {
                        lookup.lookup_type(#type_name)
                    }
                }
            ),
            _ => error(),
        }
    })
}

fn sqlite_tokens(
    attrs: &[syn::Attribute],
    generics: &syn::Generics,
    struct_ty: &syn::Ty,
) -> Option<Tokens> {
    if cfg!(not(feature = "sqlite")) {
        return None;
    }

    ident_value_of_attr_with_name(attrs, "sqlite_type").map(|ty| {
        quote!(
            impl #generics diesel::types::HasSqlType<#struct_ty> for diesel::sqlite::Sqlite {
                fn metadata(_: &()) -> diesel::sqlite::SqliteType {
                    diesel::sqlite::SqliteType::#ty
                }
            }
        )
    })
}

fn mysql_tokens(
    attrs: &[syn::Attribute],
    generics: &syn::Generics,
    struct_ty: &syn::Ty,
) -> Option<Tokens> {
    if cfg!(not(feature = "mysql")) {
        return None;
    }

    ident_value_of_attr_with_name(attrs, "mysql_type").map(|ty| {
        quote!(
            impl #generics diesel::types::HasSqlType<#struct_ty> for diesel::mysql::Mysql {
                fn metadata(_: &()) -> diesel::mysql::MysqlType {
                    diesel::mysql::MysqlType::#ty
                }
            }
        )
    })
}
