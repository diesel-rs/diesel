use super::data_structures::*;
use heck::CamelCase;
use std::error::Error;
use std::io::{stderr, Write};

pub fn determine_column_type(
    attr: &ColumnInformation,
    default_schema: String,
) -> Result<ColumnType, Box<dyn Error + Send + Sync + 'static>> {
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
        writeln!(
            &mut stderr(),
            "The column `{}` is of type `{}[]`. This will cause problems when using Diesel. You should consider changing the column type to `text[]`.",
            attr.column_name,
            tpe
        )?;
    }

    Ok(ColumnType {
        schema: attr.type_schema.as_ref().and_then(|s| {
            if s == &default_schema {
                None
            } else {
                Some(s.clone())
            }
        }),
        sql_name: tpe.to_lowercase(),
        rust_name: tpe.to_camel_case(),
        is_array,
        is_nullable: attr.nullable,
        is_unsigned: false,
    })
}
