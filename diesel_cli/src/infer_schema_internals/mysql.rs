use diesel::mysql::Mysql;
use diesel::*;
use heck::CamelCase;
use std::error::Error;

use super::data_structures::*;
use super::information_schema::UsesInformationSchema;
use super::table_data::TableName;

mod information_schema {
    use diesel::prelude::{allow_tables_to_appear_in_same_query, table};

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
        ))
        .load::<(TableName, TableName, String, _)>(connection)?
        .into_iter()
        .map(
            |(mut child_table, mut parent_table, foreign_key, primary_key)| {
                child_table.strip_schema_if_matches(&default_schema);
                parent_table.strip_schema_if_matches(&default_schema);

                ForeignKeyConstraint {
                    child_table,
                    parent_table,
                    foreign_key: foreign_key.clone(),
                    foreign_key_rust_name: foreign_key,
                    primary_key,
                }
            },
        )
        .collect();
    Ok(constraints)
}

pub fn determine_column_type(
    attr: &ColumnInformation,
) -> Result<ColumnType, Box<dyn Error + Send + Sync + 'static>> {
    let tpe = determine_type_name(&attr.type_name)?;
    let unsigned = determine_unsigned(&attr.type_name);

    Ok(ColumnType {
        schema: None,
        sql_name: tpe.trim().to_lowercase(),
        rust_name: tpe.trim().to_camel_case(),
        is_array: false,
        is_nullable: attr.nullable,
        is_unsigned: unsigned,
    })
}

fn determine_type_name(
    sql_type_name: &str,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
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
        Err(format!("unrecognized type {:?}", result).into())
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
