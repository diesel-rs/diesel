use std::borrow::Cow;

use diesel::deserialize::FromStaticSqlRow;
use diesel::query_dsl::methods::LoadQuery;
use diesel::*;
use heck::ToUpperCamelCase;

use super::data_structures::*;
use super::table_data::TableName;
use crate::infer_schema_internals::information_schema::DefaultSchema;
use crate::print_schema::ColumnSorting;

#[diesel::declare_sql_function]
extern "SQL" {
    #[sql_name = "NULLIF"]
    fn null_if_text(
        lhs: sql_types::Text,
        rhs: sql_types::Text,
    ) -> sql_types::Nullable<sql_types::Text>;
}

impl<ST, DB: mysql_like::MysqlLikeBackend> Queryable<ST, DB> for ColumnInformation
where
    (
        String,
        String,
        Option<String>,
        String,
        Option<u64>,
        Option<String>,
    ): FromStaticSqlRow<ST, DB>,
{
    type Row = (
        String,
        String,
        Option<String>,
        String,
        Option<u64>,
        Option<String>,
    );

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(ColumnInformation::new(
            row.0,
            row.1,
            row.2,
            row.3 == "YES",
            row.4,
            row.5,
        ))
    }
}

mod column_query {
    use diesel::helper_types::AsExprOf;
    use diesel::sql_types::{Nullable, Text};
    use diesel::{ExpressionMethods, IntoSql, QueryDsl, dsl};

    use super::information_schema::columns::dsl::*;
    use super::null_if_text;

    #[dsl::auto_type(type_case = "PascalCase")]
    pub(super) fn query<'a>(table: &'a str, schema: &'a str) -> _ {
        let type_schema: AsExprOf<Option<String>, Nullable<Text>> =
            None::<String>.into_sql::<Nullable<Text>>();
        columns
            .select((
                column_name,
                column_type,
                type_schema,
                __is_nullable,
                character_maximum_length,
                // MySQL comments are not nullable and are empty strings if not set
                null_if_text(column_comment, ""),
            ))
            .filter(table_name.eq(table))
            .filter(table_schema.eq(schema))
    }
}

mod comment_query {
    use diesel::{ExpressionMethods, QueryDsl, dsl};

    use super::information_schema::tables::dsl::*;

    #[dsl::auto_type(type_case = "PascalCase")]
    pub(super) fn query<'a>(table: &'a str, schema: &'a str) -> _ {
        tables
            .select(table_comment)
            .filter(table_name.eq(table))
            .filter(table_schema.eq(schema))
    }
}

pub fn get_table_data<DB>(
    conn: &mut mysql_like::MysqlLikeConnection<DB>,
    table: &TableName,
    column_sorting: &ColumnSorting,
) -> QueryResult<Vec<ColumnInformation>>
where
    DB: mysql_like::MysqlLikeBackend + DefaultSchema,
    for<'a> diesel::helper_types::Order<
        column_query::Query<'a>,
        information_schema::columns::ordinal_position,
    >: LoadQuery<'a, mysql_like::MysqlLikeConnection<DB>, ColumnInformation>,
    for<'a> diesel::helper_types::Order<column_query::Query<'a>, information_schema::columns::column_name>:
        LoadQuery<'a, mysql_like::MysqlLikeConnection<DB>, ColumnInformation>,
{
    use information_schema::columns::dsl::*;

    let schema_name = match table.schema {
        Some(ref name) => Cow::Borrowed(name),
        None => Cow::Owned(DB::default_schema(conn)?),
    };

    let query = column_query::query(&table.sql_name, &schema_name);
    let mut table_columns = match column_sorting {
        ColumnSorting::OrdinalPosition => query.order(ordinal_position).load(conn),
        ColumnSorting::Name => query.order(column_name).load(conn),
    }?;
    for c in &mut table_columns {
        if c.max_length.is_some() && !c.type_name.contains('(') {
            // Mysql returns something in character_maximum_length regardless
            // of whether it's specified at field creation time
            // In addition there is typically a shared limitation at row level,
            // so it's typically not even the real max.
            // This basically means no max.
            // https://dev.mysql.com/doc/refman/8.0/en/column-count-limit.html
            // https://chartio.com/resources/tutorials/understanding-strorage-sizes-for-mysql-text-data-types/
            c.max_length = None;
        }
    }
    Ok(table_columns)
}

pub(in crate::infer_schema_internals) mod information_schema {
    use diesel::prelude::{allow_tables_to_appear_in_same_query, table};

    table! {
        information_schema.tables (table_schema, table_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            table_comment -> VarChar,
        }
    }

    table! {
        information_schema.key_column_usage (constraint_schema, constraint_name) {
            constraint_schema -> VarChar,
            constraint_name -> VarChar,
            table_schema -> VarChar,
            table_name -> VarChar,
            column_name -> VarChar,
            referenced_table_schema -> VarChar,
            referenced_table_name -> VarChar,
            referenced_column_name -> VarChar,
        }
    }

    table! {
        information_schema.columns (table_schema, table_name, column_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            column_name -> VarChar,
            #[sql_name = "is_nullable"]
            __is_nullable -> VarChar,
            character_maximum_length -> Nullable<Unsigned<BigInt>>,
            ordinal_position -> Unsigned<BigInt>,
            udt_name -> VarChar,
            udt_schema -> VarChar,
            column_type -> VarChar,
            column_comment -> VarChar,
        }
    }

    table! {
        information_schema.table_constraints (constraint_schema, constraint_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            constraint_schema -> VarChar,
            constraint_name -> VarChar,
            constraint_type -> VarChar,
        }
    }

    allow_tables_to_appear_in_same_query!(table_constraints, key_column_usage);
}

#[tracing::instrument]
pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, crate::errors::Error> {
    let tpe = determine_type_name(&attr.type_name)?;
    let unsigned = determine_unsigned(&attr.type_name);

    Ok(ColumnType {
        schema: None,
        sql_name: tpe.trim().to_string(),
        rust_name: tpe.trim().to_upper_camel_case(),
        is_array: false,
        is_nullable: attr.nullable,
        is_unsigned: unsigned,
        record: None,
        max_length: attr.max_length,
        unmodified_type: attr.type_name.clone(),
    })
}

pub(super) fn determine_type_name(sql_type_name: &str) -> Result<String, crate::errors::Error> {
    let result = if sql_type_name == "tinyint(1)" {
        "bool"
    } else if sql_type_name.starts_with("int") {
        "integer"
    } else if let Some(idx) = sql_type_name.find('(') {
        &sql_type_name[..idx]
    } else {
        sql_type_name
    };

    if determine_unsigned(result) {
        Ok(result
            .to_lowercase()
            .replace("unsigned", "")
            .trim()
            .to_owned())
    } else if result.contains(' ') {
        Err(crate::errors::Error::UnsupportedType(result.into()))
    } else {
        Ok(result.to_owned())
    }
}

pub(super) fn determine_unsigned(sql_type_name: &str) -> bool {
    sql_type_name.to_lowercase().contains("unsigned")
}

pub fn get_enum_variants(ct: &ColumnType) -> Option<Vec<EnumVariant>> {
    if let Some(enum_variants) = ct.unmodified_type.strip_prefix("enum('")
        && let Some(enum_variants) = enum_variants.strip_suffix("')")
    {
        Some(
            enum_variants
                .split("','")
                .enumerate()
                .map(|(idx, v)| EnumVariant {
                    order: idx as _,
                    sql_name: v.replace("''", "'"),
                })
                .collect(),
        )
    } else {
        None
    }
}

pub fn get_table_comment<DB>(
    conn: &mut mysql_like::MysqlLikeConnection<DB>,
    table: &TableName,
) -> QueryResult<Option<String>>
where
    DB: mysql_like::MysqlLikeBackend + DefaultSchema,
    for<'a> comment_query::Query<'a>: LoadQuery<'a, mysql_like::MysqlLikeConnection<DB>, String>,
{
    let schema_name = match table.schema {
        Some(ref name) => Cow::Borrowed(name),
        None => Cow::Owned(DB::default_schema(conn)?),
    };

    let comment = comment_query::query(&table.sql_name, &schema_name).get_result(conn)?;

    if comment.is_empty() {
        Ok(None)
    } else {
        Ok(Some(comment))
    }
}
