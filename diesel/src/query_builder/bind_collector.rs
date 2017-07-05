use backend::{Backend, TypeMetadata};
use result::Error::SerializationError;
use result::QueryResult;
use types::{HasSqlType, ToSql, IsNull};

pub trait BindCollector<DB> {
    fn push_bound_value<T, U>(&mut self, bind: &U, lookup: &DB::MetadataLookup) -> QueryResult<()> where
        DB: Backend + HasSqlType<T>,
        U: ToSql<T, DB>;
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
    fn push_bound_value<T, U>(&mut self, bind: &U, lookup: &DB::MetadataLookup) -> QueryResult<()> where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>,
    {
        let mut bytes = Vec::new();
        match bind.to_sql(&mut bytes, lookup).map_err(SerializationError)? {
            IsNull::No => self.binds.push(Some(bytes)),
            IsNull::Yes => self.binds.push(None),
        }
        self.metadata.push(<DB as HasSqlType<T>>::metadata());
        Ok(())
    }
}
