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

fn read_type_names(csl: &str) -> HashSet<String> {
    csl.split(",")
        .map(|t| t.trim())
        .map(canonicalize_pg_type_name)
        .filter(|t| !t.is_empty()).collect()
}

fn infer_enums_for_schema_name(database_url: &str, schema_name: Option<&str>,
                               type_list: Option<&str>, camel_case_types: bool,
                               camel_case_variants: bool) -> quote::Tokens {
    let acceptable_type_names: HashSet<String> =
        type_list.map(read_type_names).unwrap_or_else(|| HashSet::new());
    let acceptable_type_name_p = |s: &str| {
        acceptable_type_names.is_empty() ||
            acceptable_type_names.contains(&canonicalize_pg_type_name(s))
    };
    let connection = establish_connection(database_url).expect("unable to connect to database");
    let inferred_enums = get_enum_information(&connection, schema_name)
        .expect("unable to read type information from database")
        .into_iter()
        .filter(|e| acceptable_type_name_p(&e.type_name))
        .map(|EnumInformation { type_name, variants, oid, array_oid }| {
            let final_type_name = if camel_case_types {
                camel_cased(&type_name)
            } else {
                type_name.clone()
            };
            let final_variants = if camel_case_variants {
                variants.into_iter().map(|s| camel_cased(&s)).collect()
            } else {
                variants
            };
            let enum_decl = generate_enum(
                &type_name, &final_type_name, &final_variants, oid, array_oid);
            match schema_name {
                None => enum_decl,
                Some(schema) => {
                    let schema = syn::Ident::new(schema);
                    quote!(mod #schema { #enum_decl })
                },
            }
        });
    quote!(#(#inferred_enums)*)
}

fn generate_enum(sql_type_name: &str, type_name: &str, variants: &[String],
                 oid: u32, array_oid: u32) -> quote::Tokens {
    let type_name = syn::Ident::new(type_name);
    let variants: Vec<syn::Ident> = variants.into_iter().map(|s| syn::Ident::new(s.as_ref())).collect();
    let has_sql_type = quote!(::diesel::types::HasSqlType<#type_name>);
    let backend = quote!(::diesel::backend::Backend);
    let box_error = quote!(
        ::std::boxed::Box<::std::error::Error + ::std::marker::Send + ::std::marker::Sync>);
    quote! {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        pub enum #type_name {
            #(#variants),*
        }

        impl ::diesel::types::ProvidesSqlTypeFor<::diesel::backend::Debug> for #type_name
            where ::diesel::backend::Debug: ::diesel::backend::TypeMetadata {
            fn self_metadata() { }
        }

        impl ::diesel::types::ProvidesSqlTypeFor<::diesel::pg::Pg> for #type_name
            where ::diesel::pg::Pg: ::diesel::backend::TypeMetadata {
            fn self_metadata() -> ::diesel::pg::PgTypeMetadata {
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

        impl<DB> ::diesel::types::ToSql<#type_name, DB> for #type_name where DB: #backend + #has_sql_type {
            fn to_sql<W: ::std::io::Write>(&self, _out: &mut W)
                                            -> ::std::result::Result<::diesel::types::IsNull, #box_error> {
                unimplemented!()
            }
        }

        impl<'a, DB> ::diesel::types::ToSql<::diesel::types::Nullable<#type_name>, DB> for #type_name
            where DB: #backend + #has_sql_type, #type_name: ::diesel::types::ToSql<#type_name, DB> {
            fn to_sql<W: ::std::io::Write>(&self, out: &mut W)
                                           -> ::std::result::Result<::diesel::types::IsNull, #box_error> {
                ::diesel::types::ToSql::<#type_name, DB>::to_sql(self, out)
            }
        }

        impl<DB> ::diesel::types::FromSql<#type_name, DB> for #type_name
            where DB: #backend + #has_sql_type {
            fn from_sql(_bytes: Option<&DB::RawValue>) -> ::std::result::Result<Self, #box_error> {
                unimplemented!()
            }
        }

        impl<DB> ::diesel::types::FromSqlRow<#type_name, DB> for #type_name
            where DB: #backend + #has_sql_type,
                  #type_name: ::diesel::types::FromSql<#type_name, DB> {
            fn build_from_row<T: ::diesel::row::Row<DB>>(row: &mut T) -> ::std::result::Result<Self, #box_error> {
                ::diesel::types::FromSql::<#type_name, DB>::from_sql(row.take())
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

        impl ::diesel::Expression for #type_name {
            type SqlType = #type_name;
        }

        impl ::diesel::query_builder::QueryFragment<::diesel::backend::Debug> for #type_name {
            fn to_sql(&self, out: &mut <::diesel::backend::Debug as #backend>::QueryBuilder)
                      -> ::diesel::query_builder::BuildQueryResult {
                use ::diesel::query_builder::QueryBuilder;
                out.push_sql(&format!(" CAST('{:?}' AS {}) ", self, #sql_type_name));
                Ok(())
            }

            fn collect_binds(&self, out: &mut <::diesel::backend::Debug as #backend>::BindCollector)
                             -> ::diesel::result::QueryResult<()> {
                Ok(())
            }

            fn is_safe_to_cache_prepared(&self) -> bool {
                true
            }
        }

        impl ::diesel::query_builder::QueryFragment<::diesel::pg::Pg> for #type_name {
            fn to_sql(&self, out: &mut <::diesel::pg::Pg as #backend>::QueryBuilder)
                      -> ::diesel::query_builder::BuildQueryResult {
                use ::diesel::query_builder::QueryBuilder;
                out.push_sql(&format!(" CAST('{:?}' AS {}) ", self, #sql_type_name));
                Ok(())
            }

            fn collect_binds(&self, out: &mut <::diesel::pg::Pg as #backend>::BindCollector)
                             -> ::diesel::result::QueryResult<()> {
                Ok(())
            }

            fn is_safe_to_cache_prepared(&self) -> bool {
                true
            }
        }

        // impl ::diesel::expression::AsExpression<#type_name> for #type_name {
        //     type Expression = ::diesel::expression::bound::Bound<#type_name, Self>;
        //     fn as_expression(self) -> Self::Expression {
        //         ::diesel::expression::bound::Bound::new(self)
        //     }
        // }

        // impl<'a, 'expr> ::diesel::expression::AsExpression<#type_name> for &'expr #type_name {
        //     type Expression = ::diesel::expression::bound::Bound<#type_name, Self>;
        //     fn as_expression(self) -> Self::Expression {
        //         ::diesel::expression::bound::Bound::new(self)
        //     }
        // }

        // impl ::diesel::expression::AsExpression<::diesel::types::Nullable<#type_name>> for #type_name {
        //     type Expression = ::diesel::expression::bound::Bound<::diesel::types::Nullable<#type_name>, Self>;
        //     fn as_expression(self) -> Self::Expression {
        //         ::diesel::expression::bound::Bound::new(self)
        //     }
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::read_type_names;

    use std::collections::HashSet;

    #[test]
    fn read_type_names_empty() {
        assert!(read_type_names("").is_empty());
        assert!(read_type_names(",,").is_empty());
        assert!(read_type_names("     ").is_empty());
        assert!(read_type_names(",  , ").is_empty());
    }

    #[test]
    fn read_type_names_single() {
        let assert_contains = |s: &str, h: HashSet<String>| {
            assert!(!h.is_empty());
            assert_eq!(canonicalize_pg_type_name(s), h.into_iter().next().unwrap());
        };
        let assert_not_contains = |s: &str, h: HashSet<String>| {
            assert!(!h.is_empty());
            assert!(s != h.iter().next().unwrap());
        };
        assert_contains("mytype", read_type_names("mytype"));
        assert_contains("MyType", read_type_names("mytype"));
        assert_not_contains("my_type", read_type_names("mytype"));
        assert_not_contains("MyType", read_type_names("my_type"));
    }

    #[test]
    fn read_type_names_multi() {
        let assert_equal = |mut ss: Vec<&str>, h: HashSet<String>| {
            ss.sort();
            let mut names: Vec<String> = h.into_iter().collect();
            names.sort();
            assert_eq!(ss, names);
        };
        assert_equal(vec!("mytype1", "mytype2"), read_type_names("mytype1,mytype2"));
        assert_equal(vec!("mytype1", "mytype2"), read_type_names(",mytype1,mytype2,"));
    }
}
