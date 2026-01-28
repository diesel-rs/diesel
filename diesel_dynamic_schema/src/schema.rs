use crate::table::Table;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
/// A database schema.
/// This type is created by the [`schema`](crate::schema()) function.
pub struct Schema<T> {
    name: T,
}

impl<'a> From<&'a str> for Schema<&'a str> {
    fn from(name: &'a str) -> Self {
        Schema::new(name)
    }
}

impl From<String> for Schema<String> {
    fn from(name: String) -> Self {
        Schema::new(name)
    }
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel_dynamic_schema::Schema;
    /// let schema: Schema<&str> = "public".into();
    /// assert_eq!(schema.name(), &"public");
    /// ```
    pub fn name(&self) -> &T {
        &self.name
    }
}
