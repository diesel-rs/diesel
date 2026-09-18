use std::borrow::Cow;
use std::collections::HashMap;

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
        String,
    ): FromStaticSqlRow<ST, DB>,
{
    type Row = (
        String,
        String,
        Option<String>,
        String,
        Option<u64>,
        Option<String>,
        String,
    );

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(ColumnInformation::new(
            row.0,
            row.1,
            row.2,
            row.3 == "YES",
            row.4,
            row.5,
            row.6.to_lowercase().contains("auto_increment"),
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
                extra,
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
            extra -> VarChar,
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

/// Groups the rows of `key_column_usage` into one constraint per foreign key.
///
/// The grouping key is the child table together with the constraint name, because MariaDB 12.1
/// no longer requires a constraint name to be unique within a database. Keying by the name alone
/// merges two unrelated keys into one constraint whose column lists are the concatenation of both,
/// which then looks like a compound key and is dropped from code generation.
pub(super) fn group_foreign_key_constraints(
    rows: Vec<(TableName, TableName, String, String, String)>,
    default_schema: &str,
) -> Vec<ForeignKeyConstraint> {
    rows.into_iter()
        .fold(
            HashMap::new(),
            |mut acc, (child_table, parent_table, foreign_key, primary_key, constraint_name)| {
                let entry = acc
                    .entry((child_table.clone(), constraint_name))
                    .or_insert_with(|| (child_table, parent_table, Vec::new(), Vec::new()));
                entry.2.push(foreign_key);
                entry.3.push(primary_key);
                acc
            },
        )
        .into_values()
        .map(
            |(mut child_table, mut parent_table, foreign_key_columns, primary_key_columns)| {
                child_table.strip_schema_if_matches(default_schema);
                parent_table.strip_schema_if_matches(default_schema);

                let foreign_key_columns_rust = foreign_key_columns
                    .iter()
                    .map(|s| super::inference::rust_name_for_sql_name(s, Some(&child_table)))
                    .collect();

                ForeignKeyConstraint {
                    child_table,
                    parent_table,
                    primary_key_columns,
                    foreign_key_columns_rust,
                    foreign_key_columns,
                }
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        child: &str,
        constraint: &str,
        foreign_key: &str,
        primary_key: &str,
    ) -> (TableName, TableName, String, String, String) {
        (
            TableName::new(child, "diesel_test"),
            TableName::new("parent", "diesel_test"),
            foreign_key.into(),
            primary_key.into(),
            constraint.into(),
        )
    }

    #[test]
    fn keys_reusing_a_constraint_name_stay_separate() {
        let mut constraints = group_foreign_key_constraints(
            vec![
                row("child_a", "fk_dup", "parent_id", "id"),
                row("child_b", "fk_dup", "parent_id", "id"),
            ],
            "diesel_test",
        );
        constraints.sort_by(|a, b| a.child_table.sql_name.cmp(&b.child_table.sql_name));

        let [a, b] = constraints.as_slice() else {
            panic!("expected two constraints, got {constraints:?}")
        };
        assert_eq!(a.child_table.sql_name, "child_a");
        assert_eq!(b.child_table.sql_name, "child_b");
        for constraint in [a, b] {
            assert_eq!(constraint.parent_table.sql_name, "parent");
            assert_eq!(constraint.foreign_key_columns, ["parent_id"]);
            assert_eq!(constraint.primary_key_columns, ["id"]);
            assert_eq!(constraint.child_table.schema, None);
            assert_eq!(constraint.parent_table.schema, None);
        }
    }

    #[test]
    fn columns_of_one_compound_key_are_collected_together() {
        let constraints = group_foreign_key_constraints(
            vec![
                row("child", "fk_compound", "parent_a", "a"),
                row("child", "fk_compound", "parent_b", "b"),
            ],
            "diesel_test",
        );

        let [constraint] = constraints.as_slice() else {
            panic!("expected one constraint, got {constraints:?}")
        };
        assert_eq!(constraint.foreign_key_columns, ["parent_a", "parent_b"]);
        assert_eq!(constraint.primary_key_columns, ["a", "b"]);
    }
}
