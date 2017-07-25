use std::error::Error;
use std::io::{stderr, Write};

use data_structures::*;
use diesel::*;
use diesel::pg::PgConnection;
use diesel::pg::types::sql_types::Array;
use diesel::types;

sql_function!(array_agg, array_agg_t, (a: types::Text) -> Array<types::Text>);

table! {
    pg_type(oid) {
        typname -> Text,
        oid -> Oid,
        typnamespace -> Oid,
    }
}

table! {
    pg_enum(enumtypid) {
        enumlabel -> Text,
        enumtypid -> Oid,
    }
}

table! {
    pg_catalog.pg_namespace(oid) {
        oid -> Oid,
        nspname -> Text,
    }
}

pub fn load_enums(database_url: &str, schema_name: Option<&str>)
                  -> Result<Vec<EnumInformation>, Box<Error>> {
    use super::inference::InferConnection;
    let conn = super::inference::establish_connection(database_url)?;
    match conn {
        InferConnection::Pg(pg) => load_enums_intern(&pg, schema_name),
        #[cfg(any(feature = "sqlite", feature = "mysql"))]
        _ => unimplemented!()
    }
}

pub fn load_enums_intern(connection: &PgConnection, schema_name: Option<&str>)
                         -> Result<Vec<EnumInformation>, Box<Error>> {
    use self::pg_type::dsl::{pg_type, typname, oid as pg_type_oid, typnamespace};
    use self::pg_enum::dsl::{pg_enum, enumlabel, enumtypid};
    use self::pg_namespace::dsl::{pg_namespace, oid as pg_namespace_oid, nspname};
    let enums: Vec<(u32, String)> = try!(
        pg_type.filter(
            typnamespace.eq_any(pg_namespace
                                .filter(nspname.eq(schema_name.unwrap_or("public")))
                                .select(pg_namespace_oid))
                .and(pg_type_oid.eq_any(pg_enum.select(enumtypid))))
            .select((pg_type_oid, typname))
            .load(connection));

    let enum_ids  = enums.iter().map(|&(ref id, ..)| *id).collect::<Vec<_>>();
    let enum_labels: Vec<Vec<String>> = try!(pg_enum
                                             .filter(enumtypid
                                                     .eq_any(enum_ids))
                                             .group_by(enumtypid)
                                             .select(array_agg(enumlabel))
                                             .load(connection));
    assert_eq!(enums.len(), enum_labels.len());
    Ok(enums.into_iter()
       .zip(enum_labels.into_iter())
       .map(|((_, enum_name), field_names)|{
           EnumInformation{
               type_name: enum_name,
               fields: field_names,
           }
       }).collect())
}

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<Error>> {
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
        writeln!(&mut stderr(), "The column `{}` is of type `{}[]`. This will cause problems when using Diesel. You should consider changing the column type to `text[]`.", attr.column_name, tpe)?;
    }

    Ok(ColumnType {
        path: vec!["diesel".into(), "types".into(), capitalize(tpe)],
        is_array: is_array,
        is_nullable: attr.nullable,
    })
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}
