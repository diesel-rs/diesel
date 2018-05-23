extern crate diesel;

mod column;
mod dummy_expression;
mod table;

pub use column::Column;
pub use table::Table;

pub fn table<T>(name: T) -> Table<T> {
    Table::new(name)
}
