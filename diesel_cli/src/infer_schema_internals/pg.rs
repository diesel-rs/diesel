use std::error::Error;
use std::io::{stderr, Write};

use super::data_structures::*;

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<dyn Error>> {
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
        rust_name: capitalize(tpe),
        is_array,
        is_nullable: attr.nullable,
        is_unsigned: false,
    })
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}
