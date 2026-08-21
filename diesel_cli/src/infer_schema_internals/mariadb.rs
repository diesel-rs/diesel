use diesel::mariadb::{Mariadb, MariadbConnection};
use diesel::*;
use std::collections::HashMap;

use super::data_structures::*;
use super::information_schema::DefaultSchema;
use super::table_data::TableName;

/// Even though this is using `information_schema`, Mariadb needs non-ANSI columns
/// in order to do this.
pub fn load_foreign_key_constraints(
    connection: &mut MariadbConnection,
    schema_name: Option<&str>,
) -> QueryResult<Vec<ForeignKeyConstraint>> {
    use super::mysql_like::information_schema::key_column_usage as kcu;
    use super::mysql_like::information_schema::table_constraints as tc;

    let default_schema = Mariadb::default_schema(connection)?;
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
                .and(tc::constraint_name.eq(kcu::constraint_name))
                .and(kcu::table_name.eq(tc::table_name))),
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
                    .entry((parent_table.clone(), fk_constraint_name))
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

#[cfg(test)]
use super::mysql_like::{
    determine_column_type, determine_type_name, determine_unsigned, get_enum_variants,
    get_table_comment, get_table_data,
};
#[cfg(test)]
use crate::print_schema::ColumnSorting;

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

    fn connection() -> MariadbConnection {
        dotenv().ok();

        let connection_url = env::var("MARIADB_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        let mut connection = MariadbConnection::establish(&connection_url).unwrap();
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
            false,
        );
        let id_without_comment =
            ColumnInformation::new("id", "varchar(255)", None, false, Some(255), None, false);
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

    #[test]
    fn get_enum_variants() {
        let mut connection = connection();
        diesel::sql_query("DROP TABLE IF EXISTS enum_tests")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query(
            "CREATE TABLE enum_tests(enum_a ENUM('a', 'b'), enum_b ENUM ('a'   ,'b''c'), no_enum INTEGER)",
        )
        .execute(&mut connection)
        .unwrap();
        let table = TableName {
            sql_name: "enum_tests".into(),
            rust_name: "enum_tests".into(),
            schema: None,
        };
        let table_data =
            get_table_data(&mut connection, &table, &ColumnSorting::OrdinalPosition).unwrap();

        let [a, b, c] = table_data.as_slice() else {
            unreachable!()
        };
        assert_eq!(a.column_name, "enum_a");
        assert_eq!(b.column_name, "enum_b");
        assert_eq!(c.column_name, "no_enum");

        let a = determine_column_type(a).unwrap();
        let b = determine_column_type(b).unwrap();
        let c = determine_column_type(c).unwrap();
        assert_eq!(a.unmodified_type, "enum('a','b')");
        assert_eq!(b.unmodified_type, "enum('a','b''c')");
        // mysql returns a slightly different type name than mariadb
        assert!(
            c.unmodified_type.starts_with("int"),
            "Unexpected type `{}`, expected a integer",
            c.unmodified_type
        );

        let enum_variants_a = super::get_enum_variants(&a).unwrap();
        assert_eq!(
            enum_variants_a,
            [
                EnumVariant {
                    order: 0,
                    sql_name: "a".into()
                },
                EnumVariant {
                    order: 1,
                    sql_name: "b".into()
                }
            ]
        );
        let enum_variants_b = super::get_enum_variants(&b).unwrap();
        assert_eq!(
            enum_variants_b,
            [
                EnumVariant {
                    order: 0,
                    sql_name: "a".into()
                },
                EnumVariant {
                    order: 1,
                    sql_name: "b'c".into()
                }
            ]
        );
        let enum_variants_c = super::get_enum_variants(&c);
        assert!(enum_variants_c.is_none());
    }
}
