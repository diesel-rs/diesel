use std::error::Error;
use std::io::{stderr, Write};

use data_structures::*;

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<Error>> {
    let is_array = attr.type_name.starts_with('_');
    let tpe = if is_array {
        &attr.type_name[1..]
    } else {
        &attr.type_name
    };

    let tpe_is_varchar = tpe.to_lowercase() == "varchar";

    // Postgres doesn't coerce varchar[] to text[] so print out a message to inform
    // the user.
    if tpe_is_varchar && is_array {
        writeln!(&mut stderr(), "The column `{}` is of type `varchar[]`. This will cause problems when using Diesel. You should consider changing the column type to `text[]`.", attr.column_name)?;
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
