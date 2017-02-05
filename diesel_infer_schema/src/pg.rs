use std::error::Error;

use diesel::*;
use diesel::query_builder::BoxedSelectStatement;
use diesel::types::Oid;
use diesel::pg::{PgConnection, Pg};

use table_data::TableData;
use super::data_structures::*;

// https://www.postgresql.org/docs/9.5/static/catalog-pg-attribute.html
table! {
    pg_attribute (attrelid) {
        attrelid -> Oid,
        attname -> VarChar,
        atttypid -> Oid,
        attnotnull -> Bool,
        attnum -> SmallInt,
        attisdropped -> Bool,
    }
}

// https://www.postgresql.org/docs/9.5/static/catalog-pg-type.html
table! {
    pg_type (oid) {
        oid -> Oid,
        typname -> VarChar,
    }
}

joinable!(pg_attribute -> pg_type (atttypid));
select_column_workaround!(pg_attribute -> pg_type (attrelid, attname, atttypid, attnotnull, attnum, attisdropped));
select_column_workaround!(pg_type -> pg_attribute (oid, typname));

// https://www.postgresql.org/docs/9.5/static/catalog-pg-index.html
table! {
    pg_index (indrelid) {
        indrelid -> Oid,
        indexrelid -> Oid,
        indkey -> Array<SmallInt>,
        indisprimary -> Bool,
    }
}

// https://www.postgresql.org/docs/9.5/static/catalog-pg-class.html
table! {
    pg_class (oid) {
        oid -> Oid,
        relname -> VarChar,
        relnamespace -> Oid,
    }
}

// https://www.postgresql.org/docs/9.5/static/catalog-pg-namespace.html
table! {
    pg_namespace (oid) {
        oid -> Oid,
        nspname -> VarChar,
    }
}

mod information_schema {
    table! {
        information_schema.tables (table_catalog, table_schema, table_name) {
            table_catalog -> VarChar,
            table_schema -> VarChar,
            table_name -> VarChar,
            table_type -> VarChar,
        }
    }

    table! {
        information_schema.schemata (schema_name) {
            schema_name -> VarChar,
        }
    }
}

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<Error>> {
    let is_array = attr.type_name.starts_with('_');
    let tpe = if is_array {
        &attr.type_name[1..]
    } else {
        &attr.type_name
    };

    Ok(ColumnType {
        path: vec!["diesel".into(), "types".into(), capitalize(tpe)],
        is_array: is_array,
        is_nullable: attr.nullable,
    })
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}

pub fn get_table_data(conn: &PgConnection, table: &TableData) -> QueryResult<Vec<ColumnInformation>> {
    use self::pg_attribute::dsl::*;
    use self::pg_type::dsl::{pg_type, typname};

    pg_attribute.inner_join(pg_type)
        .select((attname, typname, attnotnull))
        .filter(attrelid.eq_any(table_oid(table)))
        .filter(attnum.gt(0).and(attisdropped.ne(true)))
        .order(attnum)
        .load(conn)
}


pub fn get_primary_keys(conn: &PgConnection, table: &TableData) -> QueryResult<Vec<String>> {
    use self::pg_attribute::dsl::*;
    use self::pg_index::dsl::{pg_index, indisprimary, indexrelid, indrelid};

    let pk_query = pg_index.select(indexrelid)
        .filter(indrelid.eq_any(table_oid(table)))
        .filter(indisprimary.eq(true));

    pg_attribute.select(attname)
        .filter(attrelid.eq_any(pk_query))
        .order(attnum)
        .load(conn)
}

fn table_oid(table: &TableData) -> BoxedSelectStatement<Oid, pg_class::table, Pg> {
    use self::pg_class::dsl::*;
    use self::pg_namespace::{table as pg_namespace, oid as nsoid, nspname};

    let schema_oid = pg_namespace.select(nsoid)
        .filter(nspname.eq(table.schema().clone()
            .expect("Postgres TableDate has schema")))
        .limit(1);
    pg_class.select(oid)
        .filter(relname.eq(table.name()))
        .filter(relnamespace.eq_any(schema_oid))
        .limit(1)
        .into_boxed()
}

pub fn load_table_names(connection: &PgConnection, schema_name: Option<&str>)
    -> Result<Vec<TableData>, Box<Error>>
{
    use self::information_schema::tables::dsl::*;

    let schema_name = schema_name.unwrap_or("public");

    let tns: Vec<String> = tables.select(table_name)
        .filter(table_schema.eq(schema_name))
        .filter(table_name.not_like("\\_\\_%"))
        .filter(table_type.like("BASE TABLE"))
        .load(connection)?;

    let tns = tns.iter().map(|n| TableData::new(n, Some(schema_name))).collect();

    Ok(tns)
}

#[cfg(test)]
extern crate dotenv;

#[test]
fn skip_views() {
    use self::dotenv::dotenv;
    dotenv().ok();

    let connection_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in order to run tests");
    let connection = PgConnection::establish(&connection_url).unwrap();
    connection.begin_test_transaction().unwrap();

    connection.execute("CREATE TABLE a_regular_table (id SERIAL PRIMARY KEY)").unwrap();
    connection.execute("CREATE VIEW a_view AS SELECT 42").unwrap();

    let table_names = load_table_names(&connection, None).unwrap();

    assert!(table_names.contains(&TableData::new("a_regular_table", Some("public"))));
    assert!(!table_names.contains(&TableData::new("a_view", Some("public"))));
}
