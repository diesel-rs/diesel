use backend::{Backend, TypeMetadata};
use types::HasSqlType;

pub trait BindCollector<DB: Backend> {
    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>) where DB: HasSqlType<T>;
}

#[derive(Debug)]
pub struct RawBytesBindCollector<DB: Backend + TypeMetadata> {
    pub metadata: Vec<DB::TypeMetadata>,
    pub binds: Vec<Option<Vec<u8>>>,
}

impl<DB: Backend + TypeMetadata> RawBytesBindCollector<DB> {
    pub fn new() -> Self {
        RawBytesBindCollector {
            metadata: Vec::new(),
            binds: Vec::new(),
        }
    }
}

impl<DB: Backend + TypeMetadata> BindCollector<DB> for RawBytesBindCollector<DB> {
    fn push_bound_value<T>(&mut self, bind: Option<Vec<u8>>) where DB: HasSqlType<T> {
        self.metadata.push(<DB as HasSqlType<T>>::metadata());
        self.binds.push(bind);
    }
}
