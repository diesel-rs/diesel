use std::error::Error;

use data_structures::*;

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
        is_unsigned: false,
    })
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}
