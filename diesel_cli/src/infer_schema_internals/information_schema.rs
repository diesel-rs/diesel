use std::borrow::Cow;

use diesel::backend::Backend;
use diesel::connection::LoadConnection;
use diesel::deserialize::FromSql;
use diesel::dsl::*;
use diesel::expression::QueryMetadata;
#[cfg(feature = "mysql")]
use diesel::mysql::Mysql;
#[cfg(feature = "postgres")]
use diesel::pg::Pg;
use diesel::query_builder::QueryFragment;
use diesel::*;

use self::information_schema::{key_column_usage, table_constraints, tables};
use super::inference;
use super::table_data::TableName;

pub trait DefaultSchema: Backend {
    fn default_schema<C>(conn: &mut C) -> QueryResult<String>
    where
        C: LoadConnection<Backend = Self>,
        String: FromSql<sql_types::Text, C::Backend>;
}

#[cfg(feature = "postgres")]
impl DefaultSchema for Pg {
    fn default_schema<C>(_conn: &mut C) -> QueryResult<String> {
        Ok("public".into())
    }
}

#[cfg(feature = "mysql")]
define_sql_function!(fn database() -> VarChar);

#[cfg(feature = "mysql")]
impl DefaultSchema for Mysql {
    fn default_schema<C>(conn: &mut C) -> QueryResult<String>
    where
        C: LoadConnection<Backend = Self>,
        String: FromSql<sql_types::Text, C::Backend>,
    {
        select(database()).get_result(conn)
    }
}

#[allow(clippy::module_inception)]
pub mod information_schema {
    use diesel::prelude::{allow_tables_to_appear_in_same_query, table};

    table! {
        information_schema.tables (table_schema, table_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            table_type -> VarChar,
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
            unique_constraint_schema -> Nullable<VarChar>,
            unique_constraint_name -> Nullable<VarChar>,
        }
    }

    allow_tables_to_appear_in_same_query!(table_constraints, referential_constraints);
    allow_tables_to_appear_in_same_query!(key_column_usage, table_constraints);
}

pub fn get_primary_keys<'a, Conn>(conn: &mut Conn, table: &'a TableName) -> QueryResult<Vec<String>>
where
    Conn: LoadConnection,
    Conn::Backend: DefaultSchema,
    String: FromSql<sql_types::Text, Conn::Backend>,
    Order<
        Filter<
            Filter<
                Filter<
                    Select<key_column_usage::table, key_column_usage::column_name>,
                    EqAny<
                        key_column_usage::constraint_name,
                        Filter<
                            Select<table_constraints::table, table_constraints::constraint_name>,
                            Eq<table_constraints::constraint_type, &'static str>,
                        >,
                    >,
                >,
                Eq<key_column_usage::table_name, &'a String>,
            >,
            Eq<key_column_usage::table_schema, Cow<'a, String>>,
        >,
        key_column_usage::ordinal_position,
    >: QueryFragment<Conn::Backend>,
    Conn::Backend: QueryMetadata<sql_types::Text> + 'static,
{
    use self::information_schema::key_column_usage::dsl::*;
    use self::information_schema::table_constraints::constraint_type;

    let pk_query = table_constraints::table
        .select(table_constraints::constraint_name)
        .filter(constraint_type.eq("PRIMARY KEY"));

    let schema_name = match table.schema {
        Some(ref name) => Cow::Borrowed(name),
        None => Cow::Owned(Conn::Backend::default_schema(conn)?),
    };

    key_column_usage
        .select(column_name)
        .filter(constraint_name.eq_any(pk_query))
        .filter(table_name.eq(&table.sql_name))
        .filter(table_schema.eq(schema_name))
        .order(ordinal_position)
        .load(conn)
}

pub fn load_table_names<'a, Conn>(
    connection: &mut Conn,
    schema_name: Option<&'a str>,
) -> Result<Vec<TableName>, crate::errors::Error>
where
    Conn: LoadConnection,
    Conn::Backend: DefaultSchema + 'static,
    String: FromSql<sql_types::Text, Conn::Backend>,
    Filter<
        Filter<
            Filter<
                Select<tables::table, tables::table_name>,
                Eq<tables::table_schema, Cow<'a, str>>,
            >,
            NotLike<tables::table_name, &'static str>,
        >,
        Like<tables::table_type, &'static str>,
    >: QueryFragment<Conn::Backend>,
    Conn::Backend: QueryMetadata<sql_types::Text>,
{
    use self::information_schema::tables::dsl::*;

    let default_schema = Conn::Backend::default_schema(connection)?;
    let db_schema_name = schema_name
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(default_schema.clone()));

    let mut table_names = tables
        .select(table_name)
        .filter(table_schema.eq(db_schema_name))
        .filter(table_name.not_like("\\_\\_%"))
        .filter(table_type.like("BASE TABLE"))
        .load::<String>(connection)?;
    table_names.sort_unstable();
    Ok(table_names
        .into_iter()
        .map(|name| TableName {
            rust_name: inference::rust_name_for_sql_name(&name),
            sql_name: name,
            schema: schema_name
                .filter(|&schema| schema != default_schema)
                .map(|schema| schema.to_owned()),
        })
        .collect())
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    extern crate dotenvy;

    use self::dotenvy::dotenv;
    use super::*;
    use std::env;

    fn connection() -> PgConnection {
        dotenv().ok();

        let connection_url = env::var("PG_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run tests");
        let mut connection = PgConnection::establish(&connection_url).unwrap();
        connection.begin_test_transaction().unwrap();
        connection
    }

    #[test]
    fn skip_views() {
        let mut connection = connection();

        diesel::sql_query("CREATE TABLE a_regular_table (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE VIEW a_view AS SELECT 42")
            .execute(&mut connection)
            .unwrap();

        let table_names = load_table_names(&mut connection, None).unwrap();

        assert!(table_names.contains(&TableName::from_name("a_regular_table")));
        assert!(!table_names.contains(&TableName::from_name("a_view")));
    }

    #[test]
    fn load_table_names_loads_from_public_schema_if_none_given() {
        let mut connection = connection();

        diesel::sql_query(
                "CREATE TABLE load_table_names_loads_from_public_schema_if_none_given (id SERIAL PRIMARY KEY)",
            ).execute(&mut connection)
            .unwrap();

        let table_names = load_table_names(&mut connection, None).unwrap();
        for TableName { schema, .. } in &table_names {
            assert_eq!(None, *schema);
        }
        assert!(table_names.contains(&TableName::from_name(
            "load_table_names_loads_from_public_schema_if_none_given",
        ),));
    }

    #[test]
    fn load_table_names_loads_from_custom_schema() {
        let mut connection = connection();

        diesel::sql_query("CREATE SCHEMA test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();

        let table_names = load_table_names(&mut connection, Some("test_schema")).unwrap();
        assert_eq!(vec![TableName::new("table_1", "test_schema")], table_names);

        diesel::sql_query("CREATE TABLE test_schema.table_2 (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();

        let table_names = load_table_names(&mut connection, Some("test_schema")).unwrap();
        let expected = vec![
            TableName::new("table_1", "test_schema"),
            TableName::new("table_2", "test_schema"),
        ];
        assert_eq!(expected, table_names);

        diesel::sql_query("CREATE SCHEMA other_test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE other_test_schema.table_1 (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();

        let table_names = load_table_names(&mut connection, Some("test_schema")).unwrap();
        let expected = vec![
            TableName::new("table_1", "test_schema"),
            TableName::new("table_2", "test_schema"),
        ];
        assert_eq!(expected, table_names);
        let table_names = load_table_names(&mut connection, Some("other_test_schema")).unwrap();
        assert_eq!(
            vec![TableName::new("table_1", "other_test_schema")],
            table_names
        );
    }

    #[test]
    fn load_table_names_output_is_ordered() {
        let mut connection = connection();
        diesel::sql_query("CREATE SCHEMA test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.ccc (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.aaa (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.bbb (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();

        let table_names = load_table_names(&mut connection, Some("test_schema"))
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
        let mut connection = connection();

        diesel::sql_query("CREATE SCHEMA test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query(
            "CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY, not_id INTEGER)",
        )
        .execute(&mut connection)
        .unwrap();
        diesel::sql_query(
                "CREATE TABLE test_schema.table_2 (id INTEGER, id2 INTEGER, not_id INTEGER, PRIMARY KEY (id, id2))",
            ).execute(&mut connection)
            .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        assert_eq!(
            vec!["id".to_string()],
            get_primary_keys(&mut connection, &table_1).unwrap()
        );
        assert_eq!(
            vec!["id".to_string(), "id2".to_string()],
            get_primary_keys(&mut connection, &table_2).unwrap()
        );
    }
}
