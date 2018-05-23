use table::Table;

#[derive(Debug, Clone, Copy)]
pub struct Schema<T> {
    name: T,
}

impl<T> Schema<T> {
    pub(crate) fn new(name: T) -> Self {
        Self { name }
    }

    pub fn table<U>(&self, name: U) -> Table<U, T>
    where
        T: Clone,
    {
        Table::with_schema(self.name.clone(), name)
    }
}
