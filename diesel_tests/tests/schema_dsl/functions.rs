use diesel::sql_types;

use super::structures::*;

pub fn create_temporary_table<Cols>(name: &str, columns: Cols) -> Temporary<CreateTable<'_, Cols>> {
    Temporary(CreateTable::new(name, columns))
}

pub fn create_table<Cols>(name: &str, columns: Cols) -> CreateTable<'_, Cols> {
    CreateTable::new(name, columns)
}

pub fn integer(name: &str) -> Column<'_, sql_types::Integer> {
    Column::new(name, "INTEGER")
}

pub fn string(name: &str) -> Column<'_, sql_types::VarChar> {
    Column::new(name, "VARCHAR(255)")
}

pub fn timestamp(name: &str) -> Column<'_, sql_types::VarChar> {
    Column::new(name, "TIMESTAMP")
}

#[cfg(feature = "postgres")]
pub fn timestamptz(name: &str) -> Column<'_, sql_types::VarChar> {
    Column::new(name, "TIMESTAMPTZ")
}

pub fn time(name: &str) -> Column<'_, sql_types::VarChar> {
    Column::new(name, "TIME")
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
pub fn time_with_fractional_seconds(name: &str) -> Column<'_, sql_types::VarChar> {
    Column::new(name, "TIME(6)")
}

pub fn date(name: &str) -> Column<'_, sql_types::VarChar> {
    Column::new(name, "DATE")
}
