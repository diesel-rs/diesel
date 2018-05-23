extern crate diesel;

mod column;
mod dummy_expression;
mod schema;
mod table;

pub use column::Column;
pub use schema::Schema;
pub use table::Table;

pub fn table<T>(name: T) -> Table<T> {
    Table::new(name)
}

pub fn schema<T>(name: T) -> Schema<T> {
    Schema::new(name)
}
