use backend::{Backend, TypeMetadata};
use result::Error::SerializationError;
use result::QueryResult;
use types::{HasSqlType, IsNull, ToSql, ToSqlOutput};

pub trait BindCollector<DB: Backend> {
    fn push_bound_value<T, U>(
        &mut self,
        bind: &U,
        metadata_lookup: &DB::MetadataLookup,
    ) -> QueryResult<()>
    where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>;
}

#[derive(Debug)]
pub struct RawBytesBindCollector<DB: Backend + TypeMetadata> {
    pub metadata: Vec<DB::TypeMetadata>,
    pub binds: Vec<Option<Vec<u8>>>,
}

impl<DB: Backend + TypeMetadata> RawBytesBindCollector<DB> {
    #[cfg_attr(feature = "clippy", allow(new_without_default_derive))]
    pub fn new() -> Self {
        RawBytesBindCollector {
            metadata: Vec::new(),
            binds: Vec::new(),
        }
    }
}

impl<DB: Backend + TypeMetadata> BindCollector<DB> for RawBytesBindCollector<DB> {
    fn push_bound_value<T, U>(
        &mut self,
        bind: &U,
        metadata_lookup: &DB::MetadataLookup,
    ) -> QueryResult<()>
    where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>,
    {
        let mut to_sql_output = ToSqlOutput::new(Vec::new(), metadata_lookup);
        match bind.to_sql(&mut to_sql_output).map_err(SerializationError)? {
            IsNull::No => self.binds.push(Some(to_sql_output.into_inner())),
            IsNull::Yes => self.binds.push(None),
        }
        self.metadata
            .push(<DB as HasSqlType<T>>::metadata(metadata_lookup));
        Ok(())
    }
}
