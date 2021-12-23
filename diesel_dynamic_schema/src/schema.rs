use crate::table::Table;

#[derive(Debug, Clone, Copy)]
/// A database schema.
/// This type is created by the [`schema`](crate::schema()) function.
pub struct Schema<T> {
    name: T,
}

impl<T> Schema<T> {
    pub(crate) fn new(name: T) -> Self {
        Self { name }
    }

    /// Create a table with this schema.
    pub fn table<U>(&self, name: U) -> Table<U, T>
    where
        T: Clone,
    {
        Table::with_schema(self.name.clone(), name)
    }

    /// Gets the name of the schema, as specified on creation.
    pub fn name(&self) -> &T {
        &self.name
    }
}
