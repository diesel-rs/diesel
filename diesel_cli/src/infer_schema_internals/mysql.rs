use diesel::deserialize::FromStaticSqlRow;
use diesel::mysql::Mysql;
use diesel::*;
use heck::ToUpperCamelCase;
use std::borrow::Cow;
use std::collections::HashMap;

use super::data_structures::*;
use super::information_schema::DefaultSchema;
use super::table_data::TableName;
use crate::print_schema::ColumnSorting;

diesel::define_sql_function! {
    #[sql_name = "NULLIF"]
    fn null_if_text(lhs: sql_types::Text, rhs: sql_types::Text) -> sql_types::Nullable<sql_types::Text>
}

pub fn get_table_data(
    conn: &mut MysqlConnection,
    table: &TableName,
    column_sorting: &ColumnSorting,
) -> QueryResult<Vec<ColumnInformation>> {
    use self::information_schema::columns::dsl::*;

    let schema_name = match table.schema {
        Some(ref name) => Cow::Borrowed(name),
        None => Cow::Owned(Mysql::default_schema(conn)?),
    };

    let type_schema = None::<String>.into_sql();
    let query = columns
        .select((
            column_name,
            column_type,
            type_schema,
            __is_nullable,
            character_maximum_length,
            // MySQL comments are not nullable and are empty strings if not set
            null_if_text(column_comment, ""),
        ))
        .filter(table_name.eq(&table.sql_name))
        .filter(table_schema.eq(schema_name));
    let mut table_columns: Vec<ColumnInformation> = match column_sorting {
        ColumnSorting::OrdinalPosition => query.order(ordinal_position).load(conn)?,
        ColumnSorting::Name => query.order(column_name).load(conn)?,
    };
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

impl<ST> Queryable<ST, Mysql> for ColumnInformation
where
    (
        String,
        String,
        Option<String>,
        String,
        Option<u64>,
        Option<String>,
    ): FromStaticSqlRow<ST, Mysql>,
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

mod information_schema {
    use diesel::prelude::{allow_tables_to_appear_in_same_query, table};

    table! {
        information_schema.tables (table_schema, table_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            table_comment -> VarChar,
        }
    }

    table! {
        information_schema.table_constraints (constraint_schema, constraint_name) {
            table_schema -> VarChar,
            constraint_schema -> VarChar,
            constraint_name -> VarChar,
            constraint_type -> VarChar,
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

    allow_tables_to_appear_in_same_query!(table_constraints, key_column_usage);
}

/// Even though this is using `information_schema`, MySQL needs non-ANSI columns
/// in order to do this.
pub fn load_foreign_key_constraints(
    connection: &mut MysqlConnection,
    schema_name: Option<&str>,
) -> QueryResult<Vec<ForeignKeyConstraint>> {
    use self::information_schema::key_column_usage as kcu;
    use self::information_schema::table_constraints as tc;

    let default_schema = Mysql::default_schema(connection)?;
    let schema_name = match schema_name {
        Some(name) => name,
        None => &default_schema,
    };

    let constraints = tc::table
        .filter(tc::constraint_type.eq("FOREIGN KEY"))
        .filter(tc::table_schema.eq(schema_name))
        .filter(kcu::referenced_column_name.is_not_null())
        .inner_join(
            kcu::table.on(tc::constraint_schema
                .eq(kcu::constraint_schema)
                .and(tc::constraint_name.eq(kcu::constraint_name))),
        )
        .select((
            (kcu::table_name, kcu::table_schema),
            (kcu::referenced_table_name, kcu::referenced_table_schema),
            kcu::column_name,
            kcu::referenced_column_name,
            kcu::constraint_name,
        ))
        .load::<(TableName, TableName, String, String, String)>(connection)?
        .into_iter()
        .fold(
            HashMap::new(),
            |mut acc, (child_table, parent_table, foreign_key, primary_key, fk_constraint_name)| {
                let entry = acc
                    .entry(fk_constraint_name)
                    .or_insert_with(|| (child_table, parent_table, Vec::new(), Vec::new()));
                entry.2.push(foreign_key);
                entry.3.push(primary_key);
                acc
            },
        )
        .into_values()
        .map(
            |(mut child_table, mut parent_table, foreign_key_columns, primary_key_columns)| {
                child_table.strip_schema_if_matches(&default_schema);
                parent_table.strip_schema_if_matches(&default_schema);

                ForeignKeyConstraint {
                    child_table,
                    parent_table,
                    primary_key_columns,
                    foreign_key_columns_rust: foreign_key_columns.clone(),
                    foreign_key_columns,
                }
            },
        )
        .collect();
    Ok(constraints)
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
        max_length: attr.max_length,
    })
}

pub fn get_table_comment(
    conn: &mut MysqlConnection,
    table: &TableName,
) -> QueryResult<Option<String>> {
    use self::information_schema::tables::dsl::*;

    let schema_name = match table.schema {
        Some(ref name) => Cow::Borrowed(name),
        None => Cow::Owned(Mysql::default_schema(conn)?),
    };

    let comment: String = tables
        .select(table_comment)
        .filter(table_name.eq(&table.sql_name))
        .filter(table_schema.eq(schema_name))
        .get_result(conn)?;

    if comment.is_empty() {
        Ok(None)
    } else {
        Ok(Some(comment))
    }
}

fn determine_type_name(sql_type_name: &str) -> Result<String, crate::errors::Error> {
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

fn determine_unsigned(sql_type_name: &str) -> bool {
    sql_type_name.to_lowercase().contains("unsigned")
}

#[test]
fn values_which_already_map_to_type_are_returned_unchanged() {
    assert_eq!("text", determine_type_name("text").unwrap());
    assert_eq!("integer", determine_type_name("integer").unwrap());
    assert_eq!("biginteger", determine_type_name("biginteger").unwrap());
}

#[test]
fn trailing_parenthesis_are_stripped() {
    assert_eq!("varchar", determine_type_name("varchar(255)").unwrap());
    assert_eq!("decimal", determine_type_name("decimal(10, 2)").unwrap());
    assert_eq!("float", determine_type_name("float(1)").unwrap());
}

#[test]
fn tinyint_is_bool_if_limit_1() {
    assert_eq!("bool", determine_type_name("tinyint(1)").unwrap());
    assert_eq!("tinyint", determine_type_name("tinyint(2)").unwrap());
}

#[test]
fn int_is_treated_as_integer() {
    assert_eq!("integer", determine_type_name("int").unwrap());
    assert_eq!("integer", determine_type_name("int(11)").unwrap());
}

#[test]
fn unsigned_types_are_supported() {
    assert!(determine_unsigned("float unsigned"));
    assert!(determine_unsigned("UNSIGNED INT"));
    assert!(determine_unsigned("unsigned bigint"));
    assert!(!determine_unsigned("bigint"));
    assert!(!determine_unsigned("FLOAT"));
    assert_eq!("float", determine_type_name("float unsigned").unwrap());
    assert_eq!("int", determine_type_name("UNSIGNED INT").unwrap());
    assert_eq!("bigint", determine_type_name("unsigned bigint").unwrap());
}

#[test]
fn types_with_space_are_not_supported() {
    assert!(determine_type_name("lol wat").is_err());
}

#[cfg(test)]
mod test {
    extern crate dotenvy;

    use self::dotenvy::dotenv;
    use super::*;
    use std::env;

    fn connection() -> MysqlConnection {
        dotenv().ok();

        let connection_url = env::var("MYSQL_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        let mut connection = MysqlConnection::establish(&connection_url).unwrap();
        connection.begin_test_transaction().unwrap();
        connection
    }

    #[test]
    fn get_table_data_loads_column_information() {
        let mut connection = connection();

        diesel::sql_query("DROP TABLE IF EXISTS table_1")
            .execute(&mut connection)
            .unwrap();
        // uses VARCHAR(255) as the type because SERIAL returned bigint on most platforms and bigint(20) on MacOS
        diesel::sql_query(
            "CREATE TABLE table_1 \
            (id VARCHAR(255) PRIMARY KEY COMMENT 'column comment') \
            COMMENT 'table comment'",
        )
        .execute(&mut connection)
        .unwrap();
        diesel::sql_query("DROP TABLE IF EXISTS table_2")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE table_2 (id VARCHAR(255) PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();

        let db = diesel::select(diesel::dsl::sql::<diesel::sql_types::Text>("DATABASE()"))
            .get_result::<String>(&mut connection)
            .unwrap();

        let table_1 = TableName::new("table_1", &db);
        let table_2 = TableName::new("table_2", &db);

        let id_with_comment = ColumnInformation::new(
            "id",
            "varchar(255)",
            None,
            false,
            Some(255),
            Some("column comment".to_string()),
        );
        let id_without_comment =
            ColumnInformation::new("id", "varchar(255)", None, false, Some(255), None);
        assert_eq!(
            Ok(vec![id_with_comment]),
            get_table_data(&mut connection, &table_1, &ColumnSorting::OrdinalPosition)
        );
        assert_eq!(
            Ok(vec![id_without_comment]),
            get_table_data(&mut connection, &table_2, &ColumnSorting::OrdinalPosition)
        );
    }

    #[test]
    fn gets_table_comment() {
        let mut connection = connection();

        diesel::sql_query("DROP TABLE IF EXISTS table_1")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE table_1 (id SERIAL PRIMARY KEY) COMMENT 'table comment'")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("DROP TABLE IF EXISTS table_2")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE table_2 (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();
        let db = diesel::select(diesel::dsl::sql::<diesel::sql_types::Text>("DATABASE()"))
            .get_result::<String>(&mut connection)
            .unwrap();

        let table_1 = TableName::new("table_1", &db);
        let table_2 = TableName::new("table_2", &db);
        assert_eq!(
            Ok(Some("table comment".to_string())),
            get_table_comment(&mut connection, &table_1)
        );
        assert_eq!(Ok(None), get_table_comment(&mut connection, &table_2));
    }
}
