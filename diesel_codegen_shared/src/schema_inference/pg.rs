use diesel::*;
use diesel::pg::PgConnection;
use std::error::Error;

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
    }
}

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<Error>> {
    let is_array = attr.type_name.starts_with("_");
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

pub fn get_table_data(conn: &PgConnection, table_name: &str) -> QueryResult<Vec<ColumnInformation>> {
    use self::pg_attribute::dsl::*;
    use self::pg_type::dsl::{pg_type, typname};
    use self::pg_class::dsl::*;

    let table_oid = pg_class.select(oid).filter(relname.eq(table_name)).limit(1);

    pg_attribute.inner_join(pg_type)
        .select((attname, typname, attnotnull))
        .filter(attrelid.eq_any(table_oid))
        .filter(attnum.gt(0).and(attisdropped.ne(true)))
        .order(attnum)
        .load(conn)
}


pub fn get_primary_keys(conn: &PgConnection, table_name: &str) -> QueryResult<Vec<String>> {
    use self::pg_attribute::dsl::*;
    use self::pg_index::dsl::{pg_index, indisprimary, indexrelid, indrelid};
    use self::pg_class::dsl::*;

    let table_oid = pg_class.select(oid).filter(relname.eq(table_name)).limit(1);

    let pk_query = pg_index.select(indexrelid)
        .filter(indrelid.eq_any(table_oid))
        .filter(indisprimary.eq(true));

    pg_attribute.select(attname)
        .filter(attrelid.eq_any(pk_query))
        .order(attnum)
        .load(conn)
}

pub fn load_table_names(connection: &PgConnection) -> Result<Vec<String>, Box<Error>> {
    use diesel::expression::dsl::sql;

    let query = select(sql::<types::VarChar>("table_name FROM information_schema.tables"))
        .filter(sql::<types::Bool>("\
            table_schema = 'public' AND \
            table_name NOT LIKE '\\_\\_%' AND \
            table_type LIKE 'BASE TABLE'\
        "));
    Ok(try!(query.load(connection)))
}

#[test]
#[cfg(feature = "dotenv")]
fn skip_views() {
    use ::dotenv::dotenv;
    dotenv().ok();

    let connection_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in order to run tests");
    let connection = PgConnection::establish(&connection_url).unwrap();
    connection.begin_test_transaction().unwrap();

    connection.execute("CREATE TABLE a_regular_table (id SERIAL PRIMARY KEY)").unwrap();
    connection.execute("CREATE VIEW a_view AS SELECT 42").unwrap();

    let table_names = load_table_names(&connection).unwrap();

    assert!(table_names.contains(&"a_regular_table".to_string()));
    assert!(!table_names.contains(&"a_view".to_string()));
}
