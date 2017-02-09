use std::error::Error;

use data_structures::*;

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<Error>> {
    let tpe = determine_type_name(&attr.type_name);

    Ok(ColumnType {
        path: vec!["diesel".into(), "types".into(), capitalize(tpe)],
        is_array: false,
        is_nullable: attr.nullable,
    })
}

fn determine_type_name(sql_type_name: &str) -> &str {
    if sql_type_name == "tinyint(1)" {
        "bool"
    } else if sql_type_name.starts_with("int") {
        "integer"
    } else if let Some(idx) = sql_type_name.find('(') {
        &sql_type_name[..idx]
    } else {
        &sql_type_name
    }
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}

#[test]
fn values_which_already_map_to_type_are_returned_unchanged() {
    assert_eq!("text", determine_type_name("text"));
    assert_eq!("integer", determine_type_name("integer"));
    assert_eq!("biginteger", determine_type_name("biginteger"));
}

#[test]
fn trailing_parenthesis_are_stripped() {
    assert_eq!("varchar", determine_type_name("varchar(255)"));
    assert_eq!("decimal", determine_type_name("decimal(10, 2)"));
    assert_eq!("float", determine_type_name("float(1)"));
}

#[test]
fn tinyint_is_bool_if_limit_1() {
    assert_eq!("bool", determine_type_name("tinyint(1)"));
    assert_eq!("tinyint", determine_type_name("tinyint(2)"));
}

#[test]
fn int_is_treated_as_integer() {
    assert_eq!("integer", determine_type_name("int"));
    assert_eq!("integer", determine_type_name("int(11)"));
}
