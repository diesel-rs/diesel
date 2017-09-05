use diesel::types;

use super::structures::*;

pub fn create_table<'a, Cols>(name: &'a str, columns: Cols) -> CreateTable<'a, Cols> {
    CreateTable::new(name, columns)
}

pub fn integer<'a>(name: &'a str) -> Column<'a, types::Integer> {
    Column::new(name, "INTEGER")
}

pub fn string<'a>(name: &'a str) -> Column<'a, types::VarChar> {
    Column::new(name, "VARCHAR")
}

pub fn timestamp<'a>(name: &'a str) -> Column<'a, types::VarChar> {
    Column::new(name, "TIMESTAMP")
}

pub fn time<'a>(name: &'a str) -> Column<'a, types::VarChar> {
    Column::new(name, "TIME")
}

pub fn date<'a>(name: &'a str) -> Column<'a, types::VarChar> {
    Column::new(name, "DATE")
}
