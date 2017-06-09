use std::error::Error;

use quote;
use syn;

use table_data::TableData;
use data_structures::{ColumnInformation, EnumInformation};
use inference::{establish_connection, get_table_data, determine_column_type, get_primary_keys,
                InferConnection};

pub fn wrap_item_in_const(const_name: syn::Ident, item: quote::Tokens) -> quote::Tokens {
    quote! {
        const #const_name: () = {
            extern crate diesel;
            #item
        };
    }
}

fn to_uppercase(ty: String) -> String {
    let mut ret = String::with_capacity(ty.len());
    let mut next_uppercase = true;
    for c in ty.chars() {
        if c == '_' {
            next_uppercase = true;
            continue;
        }
        if next_uppercase {
            ret += &c.to_uppercase().to_string();
        } else {
            ret.push(c);
        }
        next_uppercase = false;
    }
    ret
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ExpandEnumMode {
    PrettyPrint,
    Codegen,
}

pub fn expand_enum(enum_info: EnumInformation, mode: ExpandEnumMode) -> quote::Tokens {
    let EnumInformation{type_name, fields, schema} = enum_info;
    let sql_mod = syn::Ident::new(format!("sql_{}", type_name));
    let sql_type_name = syn::Ident::new(to_uppercase(format!("{}Sql", type_name)));
    let type_name_string = type_name.clone();
    let type_name = syn::Ident::new(to_uppercase(type_name));

    let field_mapping = fields.clone().into_iter().map(|field| {
        let ident = syn::Ident::new(to_uppercase(field.clone()));
        quote!(#field => #type_name::#ident)
    }).collect::<Vec<_>>();
    let reverse_mapping = fields.clone().into_iter().map(|field| {
        let ident = syn::Ident::new(to_uppercase(field.clone()));
        quote!(#type_name::#ident => #field)
    }).collect::<Vec<_>>();

    let fields = fields.into_iter().map(|field|{
        syn::Ident::new(to_uppercase(field))
    });

        let map_names = {
        let from_1 = {
            let field_mapping = field_mapping.clone();
            quote! {
                impl<'a> From<&'a str> for #type_name {
                    fn from(s: &'a str) -> Self {
                        match s {
                            #(#field_mapping,)*
                            _ => unreachable!(),
                        }
                    }
                }
            }
        };
        quote!{
            #from_1
            impl From<String> for #type_name {
                fn from(s: String) -> Self {
                    match s.as_str() {
                        #(#field_mapping,)*
                        _ => unreachable!(),
                    }
                }
            }
            impl<'a> From<&'a #type_name> for String {
                fn from(t: &'a #type_name) -> Self {
                    match *t {
                        #(#reverse_mapping,)*
                    }.into()
                }
            }
        }
    };

    let metadata = quote!{
        impl HasSqlType<#sql_type_name> for Debug {
            fn metadata() {}
        }

        impl HasSqlType<#sql_type_name> for Pg {
            fn metadata() -> PgTypeMetadata {
                PgTypeMetadata::Dynamic {
                    schema: #schema,
                    typename: #type_name_string,
                    as_array: IsArray::No,
                }
            }
        }
        impl NotNull for #sql_type_name {}

        impl QueryId for #sql_type_name {
            type QueryId = Self;

            fn has_static_query_id() -> bool {
                true
            }
        }
    };

    let expressions = quote!{
        impl<'a> AsExpression<#sql_type_name> for #type_name {
            type Expression = Bound<#sql_type_name, Self>;

            fn as_expression(self) -> Self::Expression {
                Bound::new(self)
            }
        }

        impl<'a, 'expr> AsExpression<#sql_type_name> for &'expr #type_name {
            type Expression = Bound<#sql_type_name, Self>;

            fn as_expression(self) -> Self::Expression {
                Bound::new(self)
            }
        }
    };

    let expressions = quote!{
        #expressions

        impl<'a> AsExpression<Nullable<#sql_type_name>> for #type_name {
            type Expression = Bound<Nullable<#sql_type_name>, Self>;

            fn as_expression(self) -> Self::Expression {
                Bound::new(self)
            }
        }

        impl<'a, 'expr> AsExpression<Nullable<#sql_type_name>> for &'expr #type_name {
            type Expression = Bound<Nullable<#sql_type_name>, Self>;

            fn as_expression(self) -> Self::Expression {
                Bound::new(self)
            }
        }
    };

    let to_sql = quote!{
        impl ToSql<Nullable<#sql_type_name>, Pg> for #type_name {
            fn to_sql<W: Write>(&self, out: &mut W, lookup: &PgConnection) -> Result<IsNull, Box<Error+Send+Sync>> {
                ToSql::<MoodSql, Pg>::to_sql(self, out, lookup)
            }
        }
    };
    let to_sql = quote!{
        #to_sql

        impl ToSql<#sql_type_name, Pg> for #type_name {
            fn to_sql<W: Write>(&self, out: &mut W, lookup: &PgConnection) -> Result<IsNull, Box<Error + Send + Sync>> {
                ToSql::<Text, Pg>::to_sql(&String::from(self), out, lookup)
            }
}
    };

    let from_sql = quote! {
        impl FromSql<#sql_type_name, Pg> for #type_name {
            fn from_sql(bytes: Option<&[u8]>) -> Result<Self, Box<Error+Send+Sync>>{
                <String as FromSql<Text, Pg>>::from_sql(bytes).map(Into::into)
            }
        }
    };
    let from_sql = quote! {
        #from_sql
        impl FromSqlRow<#sql_type_name, Pg> for #type_name {
            fn build_from_row<R: Row<Pg>>(row: &mut R) -> Result<Self, Box<Error + Send + Sync>> {
                FromSql::<#sql_type_name, Pg>::from_sql(row.take())
            }
        }
        impl FromSqlRow<(#sql_type_name,), Pg> for #type_name {
            fn build_from_row<R: Row<Pg>>(row: &mut R) -> Result<Self, Box<Error + Send + Sync>> {
                FromSql::<#sql_type_name, Pg>::from_sql(row.take())
            }
        }
    };

    let queryable = quote!{
        impl Queryable<#sql_type_name, Pg> for #type_name {
            type Row = Self;

            fn build(row: Self::Row) -> Self {
                row
            }
        }

        impl Queryable<(#sql_type_name,),  Pg> for #type_name {
            type Row = Self;

            fn build(row: Self::Row) -> Self {
                row
            }
        }
    };
    let enum_def =     quote!{
        #[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, PartialOrd, Ord)]
        pub enum #type_name {
            #(#fields,)*
        }
        #map_names
    };
    let diesel_imports = quote!{
        use diesel::pg::{PgTypeMetadata, Pg, IsArray, PgConnection};
        use diesel::types::{HasSqlType, NotNull, IsNull};
        use diesel::types::{FromSql, FromSqlRow, ToSql};
        use diesel::types::{Nullable, Text};
        use diesel::expression::AsExpression;
    };
    let diesel_imports = quote!{
        #diesel_imports
        use diesel::row::Row;
        use diesel::query_builder::QueryId;
        use diesel::expression::bound::Bound;
        use diesel::Queryable;
        use diesel::backend::Debug;
    };
    let sql_mod_inner = quote!(
        #diesel_imports
        use super::#type_name;
        use std::error::Error;
        use std::io::Write;
        #metadata
        #expressions
        #to_sql
        #from_sql
        #queryable
    );

    if mode == ExpandEnumMode::PrettyPrint {
        quote!{
            #enum_def
            pub mod #sql_mod {
                pub struct #sql_type_name;
                #sql_mod_inner
            }
        }
    } else {
        let name = syn::Ident::new(
            format!("_IMPL_ENUM_FOR_{}",
                    type_name.as_ref().to_uppercase()));
        let sql_mod_inner = wrap_item_in_const(name, quote!{
            #sql_mod_inner
        });
        quote!{
            #enum_def
            pub mod #sql_mod {
                pub struct #sql_type_name;
                #sql_mod_inner
            }
        }
    }
}

pub fn expand_infer_table_from_schema(database_url: &str, table: &TableData)
    -> Result<quote::Tokens, Box<Error>>
{
    let connection = establish_connection(database_url)?;
    let data = get_table_data(&connection, table)?;
    let primary_keys = get_primary_keys(&connection, table)?
        .into_iter()
        .map(syn::Ident::new);
    let table_name = syn::Ident::new(&*table.name);

    let mut tokens = Vec::with_capacity(data.len());

    for a in data {
        tokens.push(column_def_tokens(table, &a, &connection)?);
    }
    let default_schema = default_schema(&connection);
    if table.schema != default_schema {
        if let Some(ref schema) = table.schema {
            let schema_name = syn::Ident::new(&schema[..]);
            return Ok(quote!(table! {
                #schema_name.#table_name (#(#primary_keys),*) {
                    #(#tokens),*,
                }
            }));
        }
    }
    Ok(quote!(table! {
        #table_name (#(#primary_keys),*) {
            #(#tokens),*,
        }
    }))
}

pub fn handle_schema<I>(tables: I, schema_name: Option<&str>) -> quote::Tokens
    where I: Iterator<Item = quote::Tokens>
{
    match schema_name {
        Some(name) => {
            let schema_ident = syn::Ident::new(name);
            quote! { pub mod #schema_ident { #(#tables)* } }
        }
        None => quote!(#(#tables)*),
    }
}

fn column_def_tokens(
    table: &TableData,
    column: &ColumnInformation,
    connection: &InferConnection,
) -> Result<quote::Tokens, Box<Error>> {
    let column_name = syn::Ident::new(&*column.column_name);
    let column_type = match determine_column_type(column, connection) {
        Ok(t) => t,
        Err(e) => return Err(format!(
            "Error determining type of {}.{}: {}",
            table,
            column.column_name,
            e,
        ).into()),
    };
    let tpe = if column_type.path[0] == "diesel" && column_type.path[1] == "types" {
        let path_segments = column_type.path
            .into_iter()
            .skip(2)
            .map(syn::PathSegment::from)
            .collect();
        syn::Path { global: false, segments: path_segments }
    } else {
        let path_segments = column_type.path
            .into_iter()
            .map(syn::PathSegment::from)
            .collect();
        syn::Path { global: true, segments: path_segments }
    };
    let mut tpe = quote!(#tpe);

    if column_type.is_array {
        tpe = quote!(Array<#tpe>);
    }
    if column_type.is_nullable {
        tpe = quote!(Nullable<#tpe>);
    }
    Ok(quote!(#column_name -> #tpe))
}

fn default_schema(conn: &InferConnection) -> Option<String> {
    #[cfg(feature="mysql")]
    use information_schema::UsesInformationSchema;
    #[cfg(feature="mysql")]
    use diesel::mysql::Mysql;

    match *conn {
        #[cfg(feature="sqlite")]
        InferConnection::Sqlite(_) => None,
        #[cfg(feature="postgres")]
        InferConnection::Pg(_) => Some("public".into()),
        #[cfg(feature="mysql")]
        InferConnection::Mysql(ref c) => Mysql::default_schema(c).ok(),
    }
}
