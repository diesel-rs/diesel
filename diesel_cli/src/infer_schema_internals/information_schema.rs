use std::error::Error;

use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::expression::NonAggregate;
#[cfg(feature = "mysql")]
use diesel::mysql::Mysql;
#[cfg(feature = "postgres")]
use diesel::pg::Pg;
use diesel::query_builder::{QueryFragment, QueryId};
use diesel::*;

use super::data_structures::*;
use super::table_data::TableName;

pub trait UsesInformationSchema: Backend {
    type TypeColumn: SelectableExpression<
            self::information_schema::columns::table,
            SqlType = sql_types::Text,
        > + NonAggregate
        + QueryId
        + QueryFragment<Self>;

    fn type_column() -> Self::TypeColumn;
    fn default_schema<C>(conn: &C) -> QueryResult<String>
    where
        C: Connection,
        String: FromSql<sql_types::Text, C::Backend>;
}

#[cfg(feature = "postgres")]
impl UsesInformationSchema for Pg {
    type TypeColumn = self::information_schema::columns::udt_name;

    fn type_column() -> Self::TypeColumn {
        self::information_schema::columns::udt_name
    }

    fn default_schema<C>(_conn: &C) -> QueryResult<String> {
        Ok("public".into())
    }
}

#[cfg(feature = "mysql")]
impl UsesInformationSchema for Mysql {
    type TypeColumn = self::information_schema::columns::column_type;

    fn type_column() -> Self::TypeColumn {
        self::information_schema::columns::column_type
    }

    fn default_schema<C>(conn: &C) -> QueryResult<String>
    where
        C: Connection,
        String: FromSql<sql_types::Text, C::Backend>,
    {
        no_arg_sql_function!(database, sql_types::VarChar);
        select(database).get_result(conn)
    }
}

#[allow(clippy::module_inception)]
mod information_schema {
    table! {
        information_schema.tables (table_schema, table_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            table_type -> VarChar,
        }
    }

    table! {
        information_schema.columns (table_schema, table_name, column_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            column_name -> VarChar,
            is_nullable -> VarChar,
            ordinal_position -> BigInt,
            udt_name -> VarChar,
            column_type -> VarChar,
        }
    }

    table! {
        information_schema.key_column_usage (table_schema, table_name, column_name, constraint_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            column_name -> VarChar,
            constraint_schema -> VarChar,
            constraint_name -> VarChar,
            ordinal_position -> BigInt,
        }
    }

    table! {
        information_schema.table_constraints (table_schema, table_name, constraint_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            constraint_schema -> VarChar,
            constraint_name -> VarChar,
            constraint_type -> VarChar,
        }
    }

    table! {
        information_schema.referential_constraints (constraint_schema, constraint_name) {
            constraint_schema -> VarChar,
            constraint_name -> VarChar,
            unique_constraint_schema -> VarChar,
            unique_constraint_name -> VarChar,
        }
    }

    allow_tables_to_appear_in_same_query!(table_constraints, referential_constraints);
    allow_tables_to_appear_in_same_query!(key_column_usage, table_constraints);
}

pub fn get_table_data<Conn>(conn: &Conn, table: &TableName) -> QueryResult<Vec<ColumnInformation>>
where
    Conn: Connection,
    Conn::Backend: UsesInformationSchema,
    String: FromSql<sql_types::Text, Conn::Backend>,
{
    use self::information_schema::columns::dsl::*;

    let schema_name = match table.schema {
        Some(ref name) => name.clone(),
        None => Conn::Backend::default_schema(conn)?,
    };

    let type_column = Conn::Backend::type_column();
    columns
        .select((column_name, type_column, is_nullable))
        .filter(table_name.eq(&table.name))
        .filter(table_schema.eq(schema_name))
        .order(ordinal_position)
        .load(conn)
}

pub fn get_primary_keys<Conn>(conn: &Conn, table: &TableName) -> QueryResult<Vec<String>>
where
    Conn: Connection,
    Conn::Backend: UsesInformationSchema,
    String: FromSql<sql_types::Text, Conn::Backend>,
{
    use self::information_schema::key_column_usage::dsl::*;
    use self::information_schema::table_constraints::{self, constraint_type};

    let pk_query = table_constraints::table
        .select(table_constraints::constraint_name)
        .filter(constraint_type.eq("PRIMARY KEY"));

    let schema_name = match table.schema {
        Some(ref name) => name.clone(),
        None => Conn::Backend::default_schema(conn)?,
    };

    key_column_usage
        .select(column_name)
        .filter(constraint_name.eq_any(pk_query))
        .filter(table_name.eq(&table.name))
        .filter(table_schema.eq(schema_name))
        .order(ordinal_position)
        .load(conn)
}

pub fn load_table_names<Conn>(
    connection: &Conn,
    schema_name: Option<&str>,
) -> Result<Vec<TableName>, Box<Error>>
where
    Conn: Connection,
    Conn::Backend: UsesInformationSchema,
    String: FromSql<sql_types::Text, Conn::Backend>,
{
    use self::information_schema::tables::dsl::*;

    let default_schema = Conn::Backend::default_schema(connection)?;
    let schema_name = match schema_name {
        Some(name) => name,
        None => &default_schema,
    };

    let mut table_names = tables
        .select((table_name, table_schema))
        .filter(table_schema.eq(schema_name))
        .filter(table_name.not_like("\\_\\_%"))
        .filter(table_type.like("BASE TABLE"))
        .order(table_name)
        .load::<TableName>(connection)?;
    for table in &mut table_names {
        table.strip_schema_if_matches(&default_schema);
    }
    Ok(table_names)
}

#[allow(clippy::similar_names)]
#[cfg(feature = "postgres")]
pub fn load_foreign_key_constraints<Conn>(
    connection: &Conn,
    schema_name: Option<&str>,
) -> QueryResult<Vec<ForeignKeyConstraint>>
where
    Conn: Connection,
    Conn::Backend: UsesInformationSchema,
    String: FromSql<sql_types::Text, Conn::Backend>,
{
    use self::information_schema::key_column_usage as kcu;
    use self::information_schema::referential_constraints as rc;
    use self::information_schema::table_constraints as tc;

    let default_schema = Conn::Backend::default_schema(connection)?;
    let schema_name = match schema_name {
        Some(name) => name,
        None => &default_schema,
    };

    let constraint_names = tc::table
        .filter(tc::constraint_type.eq("FOREIGN KEY"))
        .filter(tc::table_schema.eq(schema_name))
        .inner_join(
            rc::table.on(tc::constraint_schema
                .eq(rc::constraint_schema)
                .and(tc::constraint_name.eq(rc::constraint_name))),
        )
        .select((
            rc::constraint_schema,
            rc::constraint_name,
            rc::unique_constraint_schema,
            rc::unique_constraint_name,
        ))
        .load::<(String, String, String, String)>(connection)?;

    constraint_names
        .into_iter()
        .map(
            |(foreign_key_schema, foreign_key_name, primary_key_schema, primary_key_name)| {
                let (mut foreign_key_table, foreign_key_column) = kcu::table
                    .filter(kcu::constraint_schema.eq(&foreign_key_schema))
                    .filter(kcu::constraint_name.eq(&foreign_key_name))
                    .select(((kcu::table_name, kcu::table_schema), kcu::column_name))
                    .first::<(TableName, _)>(connection)?;
                let (mut primary_key_table, primary_key_column) = kcu::table
                    .filter(kcu::constraint_schema.eq(primary_key_schema))
                    .filter(kcu::constraint_name.eq(primary_key_name))
                    .select(((kcu::table_name, kcu::table_schema), kcu::column_name))
                    .first::<(TableName, _)>(connection)?;

                foreign_key_table.strip_schema_if_matches(&default_schema);
                primary_key_table.strip_schema_if_matches(&default_schema);

                Ok(ForeignKeyConstraint {
                    child_table: foreign_key_table,
                    parent_table: primary_key_table,
                    foreign_key: foreign_key_column,
                    primary_key: primary_key_column,
                })
            },
        )
        .collect()
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    extern crate dotenv;

    use self::dotenv::dotenv;
    use super::*;
    use std::env;

    fn connection() -> PgConnection {
        let _ = dotenv();

        let connection_url = env::var("PG_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        let connection = PgConnection::establish(&connection_url).unwrap();
        connection.begin_test_transaction().unwrap();
        connection
    }

    #[test]
    fn skip_views() {
        let connection = connection();

        connection
            .execute("CREATE TABLE a_regular_table (id SERIAL PRIMARY KEY)")
            .unwrap();
        connection
            .execute("CREATE VIEW a_view AS SELECT 42")
            .unwrap();

        let table_names = load_table_names(&connection, None).unwrap();

        assert!(table_names.contains(&TableName::from_name("a_regular_table")));
        assert!(!table_names.contains(&TableName::from_name("a_view")));
    }

    #[test]
    fn load_table_names_loads_from_public_schema_if_none_given() {
        let connection = connection();

        connection
            .execute(
                "CREATE TABLE load_table_names_loads_from_public_schema_if_none_given (id SERIAL PRIMARY KEY)",
            )
            .unwrap();

        let table_names = load_table_names(&connection, None).unwrap();
        for &TableName { ref schema, .. } in &table_names {
            assert_eq!(None, *schema);
        }
        assert!(table_names.contains(&TableName::from_name(
            "load_table_names_loads_from_public_schema_if_none_given",
        ),));
    }

    #[test]
    fn load_table_names_loads_from_custom_schema() {
        let connection = connection();

        connection.execute("CREATE SCHEMA test_schema").unwrap();
        connection
            .execute("CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY)")
            .unwrap();

        let table_names = load_table_names(&connection, Some("test_schema")).unwrap();
        assert_eq!(vec![TableName::new("table_1", "test_schema")], table_names);

        connection
            .execute("CREATE TABLE test_schema.table_2 (id SERIAL PRIMARY KEY)")
            .unwrap();

        let table_names = load_table_names(&connection, Some("test_schema")).unwrap();
        let expected = vec![
            TableName::new("table_1", "test_schema"),
            TableName::new("table_2", "test_schema"),
        ];
        assert_eq!(expected, table_names);

        connection
            .execute("CREATE SCHEMA other_test_schema")
            .unwrap();
        connection
            .execute("CREATE TABLE other_test_schema.table_1 (id SERIAL PRIMARY KEY)")
            .unwrap();

        let table_names = load_table_names(&connection, Some("test_schema")).unwrap();
        let expected = vec![
            TableName::new("table_1", "test_schema"),
            TableName::new("table_2", "test_schema"),
        ];
        assert_eq!(expected, table_names);
        let table_names = load_table_names(&connection, Some("other_test_schema")).unwrap();
        assert_eq!(
            vec![TableName::new("table_1", "other_test_schema")],
            table_names
        );
    }

    #[test]
    fn load_table_names_output_is_ordered() {
        let connection = connection();
        connection.execute("CREATE SCHEMA test_schema").unwrap();
        connection
            .execute("CREATE TABLE test_schema.ccc (id SERIAL PRIMARY KEY)")
            .unwrap();
        connection
            .execute("CREATE TABLE test_schema.aaa (id SERIAL PRIMARY KEY)")
            .unwrap();
        connection
            .execute("CREATE TABLE test_schema.bbb (id SERIAL PRIMARY KEY)")
            .unwrap();

        let table_names = load_table_names(&connection, Some("test_schema"))
            .unwrap()
            .iter()
            .map(|table| table.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            vec!["test_schema.aaa", "test_schema.bbb", "test_schema.ccc"],
            table_names
        );
    }

    #[test]
    fn get_primary_keys_only_includes_primary_key() {
        let connection = connection();

        connection.execute("CREATE SCHEMA test_schema").unwrap();
        connection
            .execute("CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY, not_id INTEGER)")
            .unwrap();
        connection
            .execute(
                "CREATE TABLE test_schema.table_2 (id INTEGER, id2 INTEGER, not_id INTEGER, PRIMARY KEY (id, id2))",
            )
            .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        assert_eq!(
            vec!["id".to_string()],
            get_primary_keys(&connection, &table_1).unwrap()
        );
        assert_eq!(
            vec!["id".to_string(), "id2".to_string()],
            get_primary_keys(&connection, &table_2).unwrap()
        );
    }

    #[test]
    fn get_table_data_loads_column_information() {
        let connection = connection();

        connection.execute("CREATE SCHEMA test_schema").unwrap();
        connection
            .execute(
                "CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY, text_col VARCHAR, not_null TEXT NOT NULL)",
            )
            .unwrap();
        connection
            .execute("CREATE TABLE test_schema.table_2 (array_col VARCHAR[] NOT NULL)")
            .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        let id = ColumnInformation::new("id", "int4", false);
        let text_col = ColumnInformation::new("text_col", "varchar", true);
        let not_null = ColumnInformation::new("not_null", "text", false);
        let array_col = ColumnInformation::new("array_col", "_varchar", false);
        assert_eq!(
            Ok(vec![id, text_col, not_null]),
            get_table_data(&connection, &table_1)
        );
        assert_eq!(Ok(vec![array_col]), get_table_data(&connection, &table_2));
    }

    #[test]
    fn get_foreign_keys_loads_foreign_keys() {
        let connection = connection();

        connection.execute("CREATE SCHEMA test_schema").unwrap();
        connection
            .execute("CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY)")
            .unwrap();
        connection
            .execute(
                "CREATE TABLE test_schema.table_2 (id SERIAL PRIMARY KEY, fk_one INTEGER NOT NULL REFERENCES test_schema.table_1)",
            )
            .unwrap();
        connection
            .execute(
                "CREATE TABLE test_schema.table_3 (id SERIAL PRIMARY KEY, fk_two INTEGER NOT NULL REFERENCES test_schema.table_2)",
            )
            .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        let table_3 = TableName::new("table_3", "test_schema");
        let fk_one = ForeignKeyConstraint {
            child_table: table_2.clone(),
            parent_table: table_1.clone(),
            foreign_key: "fk_one".into(),
            primary_key: "id".into(),
        };
        let fk_two = ForeignKeyConstraint {
            child_table: table_3.clone(),
            parent_table: table_2.clone(),
            foreign_key: "fk_two".into(),
            primary_key: "id".into(),
        };
        assert_eq!(
            Ok(vec![fk_one, fk_two]),
            load_foreign_key_constraints(&connection, Some("test_schema"))
        );
    }
}
