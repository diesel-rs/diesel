//! Types related to managing bind parameters during query construction.

use crate::backend::Backend;
use crate::result::Error::SerializationError;
use crate::result::QueryResult;
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::{HasSqlType, TypeMetadata};

/// A type which manages serializing bind parameters during query construction.
///
/// The only reason you would ever need to interact with this trait is if you
/// are adding support for a new backend to Diesel. Plugins which are extending
/// the query builder will use [`AstPass::push_bind_param`] instead.
///
/// [`AstPass::push_bind_param`]: ../struct.AstPass.html#method.push_bind_param
pub trait BindCollector<DB: Backend> {
    /// Serializes the given bind value, and collects the result.
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
/// A bind collector used by backends which transmit bind parameters as an
/// opaque blob of bytes.
///
/// For most backends, this is the concrete implementation of `BindCollector`
/// that should be used.
pub struct RawBytesBindCollector<DB: Backend + TypeMetadata> {
    /// The metadata associated with each bind parameter.
    ///
    /// This vec is guaranteed to be the same length as `binds`.
    pub metadata: Vec<DB::TypeMetadata>,
    /// The serialized bytes for each bind parameter.
    ///
    /// This vec is guaranteed to be the same length as `metadata`.
    pub binds: Vec<Option<Vec<u8>>>,
}

#[allow(clippy::new_without_default_derive)]
impl<DB: Backend + TypeMetadata> RawBytesBindCollector<DB> {
    /// Construct an empty `RawBytesBindCollector`
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
        let mut to_sql_output = Output::new(Vec::new(), metadata_lookup);
        let is_null = bind
            .to_sql(&mut to_sql_output)
            .map_err(SerializationError)?;
        let bytes = to_sql_output.into_inner();
        let metadata = <DB as HasSqlType<T>>::metadata(metadata_lookup);
        match is_null {
            IsNull::No => self.binds.push(Some(bytes)),
            IsNull::Yes => self.binds.push(None),
        }
        self.metadata.push(metadata);
        Ok(())
    }
}
