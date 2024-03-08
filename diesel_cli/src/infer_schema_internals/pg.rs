use super::data_structures::*;
use super::information_schema::DefaultSchema;
use super::TableName;
use crate::print_schema::ColumnSorting;
use diesel::connection::DefaultLoadingMode;
use diesel::deserialize::{self, FromStaticSqlRow};
use diesel::dsl::AsExprOf;
use diesel::expression::AsExpression;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{self, Array, Text};
use heck::ToUpperCamelCase;
use std::borrow::Cow;

#[tracing::instrument]
pub fn determine_column_type(
    attr: &ColumnInformation,
    default_schema: String,
) -> Result<ColumnType, crate::errors::Error> {
    let is_array = attr.type_name.starts_with('_');
    let tpe = if is_array {
        &attr.type_name[1..]
    } else {
        &attr.type_name
    };

    let diesel_alias_without_postgres_coercion = match &*tpe.to_lowercase() {
        "varchar" | "citext" => Some(tpe),
        _ => None,
    };

    // Postgres doesn't coerce varchar[] to text[] so print out a message to inform
    // the user.
    if let (true, Some(tpe)) = (is_array, diesel_alias_without_postgres_coercion) {
        tracing::info!("Cannot coerce varchar[] into text[]");
        eprintln!(
            "The column `{}` is of type `{}[]`. This will cause problems when using Diesel. You should consider changing the column type to `text[]`.",
            attr.column_name,
            tpe
        );
    }

    Ok(ColumnType {
        schema: attr.type_schema.as_ref().and_then(|s| {
            if s == &default_schema {
                None
            } else {
                Some(s.clone())
            }
        }),
        sql_name: tpe.to_string(),
        rust_name: tpe.to_upper_camel_case(),
        is_array,
        is_nullable: attr.nullable,
        is_unsigned: false,
        max_length: attr.max_length,
    })
}

diesel::postfix_operator!(Regclass, "::regclass", sql_types::Oid, backend: Pg);

fn regclass(table: &TableName) -> Regclass<AsExprOf<String, sql_types::Text>> {
    let table_name = match table.schema {
        Some(ref schema_name) => format!("\"{}\".\"{}\"", schema_name, table.sql_name),
        None => format!("\"{}\"", table.sql_name),
    };

    Regclass::new(<String as AsExpression<sql_types::Text>>::as_expression(
        table_name,
    ))
}

diesel::define_sql_function!(fn col_description(table: sql_types::Oid, column_number: sql_types::BigInt) -> sql_types::Nullable<sql_types::Text>);

pub fn get_table_data(
    conn: &mut PgConnection,
    table: &TableName,
    column_sorting: &ColumnSorting,
) -> QueryResult<Vec<ColumnInformation>> {
    use self::information_schema::columns::dsl::*;

    let schema_name = match table.schema {
        Some(ref name) => Cow::Borrowed(name),
        None => Cow::Owned(Pg::default_schema(conn)?),
    };

    let query = columns
        .select((
            column_name,
            udt_name,
            udt_schema.nullable(),
            __is_nullable,
            character_maximum_length,
            col_description(regclass(table), ordinal_position),
        ))
        .filter(table_name.eq(&table.sql_name))
        .filter(table_schema.eq(schema_name));
    match column_sorting {
        ColumnSorting::OrdinalPosition => query.order(ordinal_position).load(conn),
        ColumnSorting::Name => query.order(column_name).load(conn),
    }
}

impl<ST> Queryable<ST, Pg> for ColumnInformation
where
    (
        String,
        String,
        Option<String>,
        String,
        Option<i32>,
        Option<String>,
    ): FromStaticSqlRow<ST, Pg>,
{
    type Row = (
        String,
        String,
        Option<String>,
        String,
        Option<i32>,
        Option<String>,
    );

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(ColumnInformation::new(
            row.0,
            row.1,
            row.2,
            row.3 == "YES",
            row.4
                .map(|n| {
                    std::convert::TryInto::try_into(n).map_err(|e| {
                        format!("Max column length can't be converted to u64: {e} (got: {n})")
                    })
                })
                .transpose()?,
            row.5,
        ))
    }
}

define_sql_function!(fn obj_description(oid: sql_types::Oid, catalog: sql_types::Text) -> Nullable<Text>);

pub fn get_table_comment(
    conn: &mut PgConnection,
    table: &TableName,
) -> QueryResult<Option<String>> {
    diesel::select(obj_description(regclass(table), "pg_class")).get_result(conn)
}

mod information_schema {
    use diesel::prelude::table;

    table! {
        information_schema.columns (table_schema, table_name, column_name) {
            table_schema -> VarChar,
            table_name -> VarChar,
            column_name -> VarChar,
            #[sql_name = "is_nullable"]
            __is_nullable -> VarChar,
            character_maximum_length -> Nullable<Integer>,
            ordinal_position -> BigInt,
            udt_name -> VarChar,
            udt_schema -> VarChar,
        }
    }
}

define_sql_function! {
    #[aggregate]
    fn array_agg(input: diesel::sql_types::Text) -> diesel::sql_types::Array<diesel::sql_types::Text>;
}

#[allow(clippy::similar_names)]
pub fn load_foreign_key_constraints(
    connection: &mut PgConnection,
    schema_name: Option<&str>,
) -> QueryResult<Vec<ForeignKeyConstraint>> {
    #[derive(QueryableByName)]
    struct ForeignKeyList {
        #[diesel(sql_type = Text)]
        self_schema: String,
        #[diesel(sql_type = Text)]
        self_table: String,
        #[diesel(sql_type = Array<Text>)]
        self_columns: Vec<String>,
        #[diesel(sql_type = Text)]
        foreign_schema: String,
        #[diesel(sql_type = Text)]
        foreign_table: String,
        #[diesel(sql_type = Array<Text>)]
        foreign_columns: Vec<String>,
    }

    let default_schema = Pg::default_schema(connection)?;
    let schema_name = schema_name.unwrap_or(&default_schema);

    diesel::sql_query(include_str!("load_foreign_keys.sql"))
        .bind::<Text, _>(schema_name)
        .load_iter::<ForeignKeyList, DefaultLoadingMode>(connection)?
        .map(|f| {
            let f = f?;
            let mut child_table = TableName::new(f.self_table, f.self_schema);
            child_table.strip_schema_if_matches(&default_schema);
            let mut parent_table = TableName::new(f.foreign_table, f.foreign_schema);
            parent_table.strip_schema_if_matches(&default_schema);

            let foreign_key_columns_rust = f
                .self_columns
                .iter()
                .map(|s| super::inference::rust_name_for_sql_name(s))
                .collect();
            Ok(ForeignKeyConstraint {
                child_table,
                parent_table,
                foreign_key_columns: f.self_columns,
                foreign_key_columns_rust,
                primary_key_columns: f.foreign_columns,
            })
        })
        .collect()
}

#[cfg(test)]
mod test {
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
    fn get_table_data_loads_column_information() {
        let mut connection = connection();

        diesel::sql_query("CREATE SCHEMA test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query(
                "CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY, text_col VARCHAR(128), not_null TEXT NOT NULL)",
            ).execute(&mut connection)
            .unwrap();
        diesel::sql_query("COMMENT ON COLUMN test_schema.table_1.id IS 'column comment'")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.table_2 (array_col VARCHAR[] NOT NULL)")
            .execute(&mut connection)
            .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        let pg_catalog = Some(String::from("pg_catalog"));
        let id = ColumnInformation::new(
            "id",
            "int4",
            pg_catalog.clone(),
            false,
            None,
            Some("column comment".to_string()),
        );
        let text_col = ColumnInformation::new(
            "text_col",
            "varchar",
            pg_catalog.clone(),
            true,
            Some(128),
            None,
        );
        let not_null =
            ColumnInformation::new("not_null", "text", pg_catalog.clone(), false, None, None);
        let array_col =
            ColumnInformation::new("array_col", "_varchar", pg_catalog, false, None, None);
        assert_eq!(
            Ok(vec![id, text_col, not_null]),
            get_table_data(&mut connection, &table_1, &ColumnSorting::OrdinalPosition)
        );
        assert_eq!(
            Ok(vec![array_col]),
            get_table_data(&mut connection, &table_2, &ColumnSorting::OrdinalPosition)
        );
    }

    #[test]
    fn gets_table_comment() {
        let mut connection = connection();

        diesel::sql_query("CREATE SCHEMA test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query(
                "CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY, text_col VARCHAR, not_null TEXT NOT NULL)",
            ).execute(&mut connection)
            .unwrap();
        diesel::sql_query("COMMENT ON TABLE test_schema.table_1 IS 'table comment'")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.table_2 (array_col VARCHAR[] NOT NULL)")
            .execute(&mut connection)
            .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        assert_eq!(
            Ok(Some("table comment".to_string())),
            get_table_comment(&mut connection, &table_1)
        );
        assert_eq!(Ok(None), get_table_comment(&mut connection, &table_2));
    }

    #[test]
    fn get_foreign_keys_loads_foreign_keys() {
        let mut connection = connection();

        diesel::sql_query("CREATE SCHEMA test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query(
                "CREATE TABLE test_schema.table_2 (id SERIAL PRIMARY KEY, fk_one INTEGER NOT NULL REFERENCES test_schema.table_1)",
            ).execute(&mut connection)
            .unwrap();
        diesel::sql_query(
                "CREATE TABLE test_schema.table_3 (id SERIAL PRIMARY KEY, fk_two INTEGER NOT NULL REFERENCES test_schema.table_2)",
            ).execute(&mut connection)
            .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        let table_3 = TableName::new("table_3", "test_schema");
        let fk_one = ForeignKeyConstraint {
            child_table: table_2.clone(),
            parent_table: table_1,
            foreign_key_columns: vec!["fk_one".into()],
            foreign_key_columns_rust: vec!["fk_one".into()],
            primary_key_columns: vec!["id".into()],
        };
        let fk_two = ForeignKeyConstraint {
            child_table: table_3,
            parent_table: table_2,
            foreign_key_columns: vec!["fk_two".into()],
            foreign_key_columns_rust: vec!["fk_two".into()],
            primary_key_columns: vec!["id".into()],
        };
        assert_eq!(
            Ok(vec![fk_one, fk_two]),
            load_foreign_key_constraints(&mut connection, Some("test_schema"))
        );
    }

    #[test]
    fn get_foreign_keys_loads_foreign_keys_with_same_name() {
        let mut connection = connection();

        diesel::sql_query("CREATE SCHEMA test_schema")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query("CREATE TABLE test_schema.table_1 (id SERIAL PRIMARY KEY)")
            .execute(&mut connection)
            .unwrap();
        diesel::sql_query(
            "CREATE TABLE test_schema.table_2 (\
                    id SERIAL PRIMARY KEY,\
                    fk_id INTEGER NOT NULL,\
                    CONSTRAINT fk FOREIGN KEY (fk_id) REFERENCES test_schema.table_1 (id))",
        )
        .execute(&mut connection)
        .unwrap();
        diesel::sql_query(
            "CREATE TABLE test_schema.table_3 (\
                    id SERIAL PRIMARY KEY,\
                    fk_id INTEGER NOT NULL,\
                    CONSTRAINT fk FOREIGN KEY (fk_id) REFERENCES test_schema.table_1 (id))",
        )
        .execute(&mut connection)
        .unwrap();

        let table_1 = TableName::new("table_1", "test_schema");
        let table_2 = TableName::new("table_2", "test_schema");
        let table_3 = TableName::new("table_3", "test_schema");
        let fk_one = ForeignKeyConstraint {
            child_table: table_2,
            parent_table: table_1.clone(),
            foreign_key_columns: vec!["fk_id".into()],
            foreign_key_columns_rust: vec!["fk_id".into()],
            primary_key_columns: vec!["id".into()],
        };
        let fk_two = ForeignKeyConstraint {
            child_table: table_3,
            parent_table: table_1,
            foreign_key_columns: vec!["fk_id".into()],
            foreign_key_columns_rust: vec!["fk_id".into()],
            primary_key_columns: vec!["id".into()],
        };
        assert_eq!(
            Ok(vec![fk_one, fk_two]),
            load_foreign_key_constraints(&mut connection, Some("test_schema"))
        );
    }
}
