use backend::{Backend, TypeMetadata};
use types::HasSqlType;

pub trait BindCollector<DB: Backend> {
    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>) where DB: HasSqlType<T>;
}

pub struct RawBytesBindCollector<DB: Backend + TypeMetadata> {
    pub binds: Vec<(DB::TypeMetadata, Option<Vec<u8>>)>,
}

impl<DB: Backend + TypeMetadata> RawBytesBindCollector<DB> {
    pub fn new() -> Self {
        RawBytesBindCollector {
            binds: Vec::new(),
        }
    }
}

impl<DB: Backend + TypeMetadata> BindCollector<DB> for RawBytesBindCollector<DB> {
    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>) where DB: HasSqlType<T> {
        let metadata = <DB as HasSqlType<T>>::metadata();
        self.binds.push((metadata, bind));
    }
}
