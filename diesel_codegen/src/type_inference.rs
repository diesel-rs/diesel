use syn;
use quote;

use diesel_codegen_shared::*;

use util::{get_options_from_input, get_option, get_optional_option};

use std::collections::HashSet;

pub fn derive_infer_enums(input: syn::MacroInput) -> quote::Tokens {
    fn bug() -> ! {
        panic!("This is a bug. Please open a Github issue with your invocation of `infer_enums!`");
    }

    let options = get_options_from_input(&input.attrs, bug).unwrap_or_else(|| bug());
    let database_url = get_option(&options, "database_url", bug);
    let schema_name = get_optional_option(&options, "schema_name");
    let types = get_optional_option(&options, "types");

    infer_enums_for_schema_name(&database_url,
                                schema_name.as_ref().map(|s| &**s),
                                types.as_ref().map(|s| &**s),
                                true, true)
}

fn canonicalize_pg_type_name(type_name: &str) -> String {
    type_name.trim().to_lowercase()
}

fn camel_cased(snake_case: &str) -> String {
    snake_case.split("_").flat_map(
        |s| s.chars().take(1).flat_map(|c| c.to_uppercase().into_iter()).chain(
            s.chars().skip(1).flat_map(|c| c.to_lowercase())
                .take_while(|&c| c != '_'))).collect()
}

fn infer_enums_for_schema_name(database_url: &str, schema_name: Option<&str>, types: Option<&str>,
                               camel_case_types: bool, camel_case_variants: bool) -> quote::Tokens {
    let mut acceptable_type_names = HashSet::new();
    if let Some(type_names) = types.map(|csl| csl.split(",").map(|t| t.trim())) {
        for type_name in type_names {
            acceptable_type_names.insert(canonicalize_pg_type_name(type_name));
        }
        if acceptable_type_names.is_empty() {
            panic!("acceptable_type_names should be non-empty if specified")
        }
    };
    let acceptable_type_name_p = |s: &str| {
        acceptable_type_names.is_empty() ||
            acceptable_type_names.contains(&canonicalize_pg_type_name(s))
    };
    let connection = establish_connection(database_url).unwrap();
    let inferred_enums = get_enum_information(&connection, schema_name).unwrap().into_iter()
        .filter(|e| acceptable_type_name_p(&e.type_name))
        .map(|EnumInformation { type_name, variants, oid, array_oid }| {
            let final_type_name = if camel_case_types {
                camel_cased(&type_name)
            } else {
                type_name
            };
            let final_variants = if camel_case_variants {
                variants.into_iter().map(|s| camel_cased(&s)).collect()
            } else {
                variants
            };
            let enum_decl = generate_enum(&final_type_name, &final_variants, oid, array_oid);
            match schema_name {
                None => enum_decl,
                Some(schema) => quote! {
                    mod #schema { #enum_decl }
                },
            }
        });
    quote!(#(#inferred_enums)*)
}

fn generate_enum(type_name: &str, variants: &[String], oid: u32, array_oid: u32) -> quote::Tokens {
    let has_sql_type = quote! {
        ::diesel::types::HasSqlType<#type_name>
    };
    let backend = quote! {
        ::diesel::backend::Backend
    };
    let box_error = quote! {
        ::std::boxed::Box<::std::error::Error + ::std::marker::Send + ::std::marker::Sync>
    };
    quote! {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        pub enum #type_name {
            #(#variants),*
        }

        impl #has_sql_type for ::diesel::backend::Debug {
            fn metadata() { }
        }

        impl #has_sql_type for ::diesel::backend::Pg {
            fn metadata() -> ::diesel::pg::PgTypeMetadata {
                ::diesel::pg::PgTypeMetadata {
                    oid: #oid,
                    array_oid: #array_oid,
                }
            }
        }

        impl ::diesel::query_builder::QueryId for #type_name {
            type QueryId = Self;

            fn has_static_query_id() -> bool {
                true
            }
        }

        impl ::diesel::types::NotNull for #type_name { }

        impl<DB> ::diesel::types::ToSql<#type_name, DB> where DB: #backend + #has_sql_type {
            fn to_sql<W: ::std::io::Write>(&self, out: &mut W)
                                            -> ::std::result::Result<IsNull, #box_error> {
                unimplemented!()
            }
        }

        impl<DB> ::diesel::types::FromSql<#type_name, DB> for #type_name
            where DB: #backend + #has_sql_type {
            fn from_sql(bytes: Option<&DB::RawValue>) -> ::std::result::Result<Self, #box_error> {
                unimplemented!()
            }
        }

        impl<DB> ::diesel::types::FromSqlRow<#type_name, DB> for #type_name
            where DB: #backend + #has_sql_type,
                  #type_name: ::diesel::types::FromSql<#type_name, DB> {
            fn build_from_row<T: Row<DB>>(row: &mut T) -> ::std::result::Result<Self, #box_error> {
                FromSql::<#type_name, DB>::from_sql(row.take())
            }
        }

        impl<DB> ::diesel::query_source::Queryable<#type_name, DB> for #type_name
            where DB: #backend + #has_sql_type,
                  (#type_name): ::diesel::types::FromSqlRow<#type_name, DB> {
            type Row = Self;
            fn build(row: Self::Row) -> Self {
                row
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::camel_cased;

    #[test]
    fn camel_cased_empty() {
        assert_eq!("", camel_cased(""));
        assert_eq!("", camel_cased("_"));
    }

    #[test]
    fn camel_cased_initial_caps() {
        assert_eq!("Cased", camel_cased("CASED"));
        assert_eq!("Cased", camel_cased("cased"));
        assert_eq!("C", camel_cased("C"));
        assert_eq!("C", camel_cased("c"));
    }

    #[test]
    fn camel_cased_underscores() {
        assert_eq!("CamelCased", camel_cased("camel_cased"));
        assert_eq!("CamelCased", camel_cased("cAmEL_CAsEd"));
        assert_eq!("CamelCased", camel_cased("camel_cased_"));
        assert_eq!("CamelCased", camel_cased("_camel_cased"));
        assert_eq!("CamelCased", camel_cased("_camel_cased_"));
        assert_eq!("CamelCased", camel_cased("_camel__cased_"));
    }

    #[test]
    fn camel_cased_i18n() {
        assert_eq!("Außerdem", camel_cased("außerdem"));
        assert_eq!("AuSSerdem", camel_cased("au_ßerdem"));
    }
}
