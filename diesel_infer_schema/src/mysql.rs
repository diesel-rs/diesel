use std::error::Error;

use data_structures::*;

pub fn determine_column_type(attr: &ColumnInformation) -> Result<ColumnType, Box<Error>> {
    let tpe = determine_type_name(&attr.type_name)?;
    let unsigned = determine_unsigned(&attr.type_name);

    Ok(ColumnType {
        path: vec!["diesel".into(), "types".into(), capitalize(tpe.trim())],
        is_array: false,
        is_nullable: attr.nullable,
        is_unsigned: unsigned,
    })
}

fn determine_type_name(sql_type_name: &str) -> Result<String, Box<Error>> {
    let result = if sql_type_name == "tinyint(1)" {
        "bool"
    } else if sql_type_name.starts_with("int") {
        "integer"
    } else if let Some(idx) = sql_type_name.find('(') {
        &sql_type_name[..idx]
    } else {
        sql_type_name
    };

    if result.to_lowercase().contains("unsigned") {
        let result = result.to_lowercase().replace("unsigned", "").trim().to_owned();
        Ok(result)
    } else if result.contains(' ') {
        Err(format!("unrecognized type {:?}", result).into())
    } else {
        Ok(result.to_owned())
    }
}

fn determine_unsigned(sql_type_name: &str) -> bool {
    sql_type_name.to_lowercase().contains("unsigned")
}

fn capitalize(name: &str) -> String {
    name[..1].to_uppercase() + &name[1..]
}

#[test]
fn values_which_already_map_to_type_are_returned_unchanged() {
    assert_eq!("text", determine_type_name("text").unwrap());
    assert_eq!("integer", determine_type_name("integer").unwrap());
    assert_eq!("biginteger", determine_type_name("biginteger").unwrap());
}

#[test]
fn trailing_parenthesis_are_stripped() {
    assert_eq!("varchar", determine_type_name("varchar(255)").unwrap());
    assert_eq!("decimal", determine_type_name("decimal(10, 2)").unwrap());
    assert_eq!("float", determine_type_name("float(1)").unwrap());
}

#[test]
fn tinyint_is_bool_if_limit_1() {
    assert_eq!("bool", determine_type_name("tinyint(1)").unwrap());
    assert_eq!("tinyint", determine_type_name("tinyint(2)").unwrap());
}

#[test]
fn int_is_treated_as_integer() {
    assert_eq!("integer", determine_type_name("int").unwrap());
    assert_eq!("integer", determine_type_name("int(11)").unwrap());
}

#[test]
fn unsigned_types_are_supported() {
    assert!(determine_unsigned("float unsigned"));
    assert!(determine_unsigned("UNSIGNED INT"));
    assert!(determine_unsigned("unsigned bigint"));
    assert!(!determine_unsigned("bigint"));
    assert!(!determine_unsigned("FLOAT"));
    assert_eq!("float", determine_type_name("float unsigned").unwrap());
    assert_eq!("int", determine_type_name("UNSIGNED INT").unwrap());
    assert_eq!("bigint", determine_type_name("unsigned bigint").unwrap());
}

#[test]
fn types_with_space_are_not_supported() {
    assert!(determine_type_name("lol wat").is_err());
}
