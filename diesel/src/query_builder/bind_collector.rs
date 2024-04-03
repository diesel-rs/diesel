//! Types related to managing bind parameters during query construction.

use crate::backend::Backend;
use crate::result::Error::SerializationError;
use crate::result::QueryResult;
use crate::serialize::{IsNull, Output, ToSql};
use crate::sql_types::{HasSqlType, TypeMetadata};

#[doc(inline)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::private::ByteWrapper;

/// A type which manages serializing bind parameters during query construction.
///
/// The only reason you would ever need to interact with this trait is if you
/// are adding support for a new backend to Diesel. Plugins which are extending
/// the query builder will use [`AstPass::push_bind_param`] instead.
///
/// [`AstPass::push_bind_param`]: crate::query_builder::AstPass::push_bind_param()
pub trait BindCollector<'a, DB: TypeMetadata>: Sized {
    /// The internal buffer type used by this bind collector
    type Buffer;

    /// Serializes the given bind value, and collects the result.
    fn push_bound_value<T, U>(
        &mut self,
        bind: &'a U,
        metadata_lookup: &mut DB::MetadataLookup,
    ) -> QueryResult<()>
    where
        DB: Backend + HasSqlType<T>,
        U: ToSql<T, DB> + ?Sized + 'a;

    /// Push a null value with the given type information onto the bind collector
    ///
    // For backward compatibility reasons we provide a default implementation
    // but custom backends that want to support `#[derive(MultiConnection)]`
    // need to provide a customized implementation of this function
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn push_null_value(&mut self, _metadata: DB::TypeMetadata) -> QueryResult<()> {
        Ok(())
    }
}

/// A movable version of the bind collector which allows it to be extracted, moved and refilled.
///
/// This is mostly useful in async context where bind data needs to be moved across threads.
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub trait MoveableBindCollector<DB: TypeMetadata> {
    /// The movable bind data of this bind collector
    type BindData: Send + 'static;

    /// Builds a movable version of the bind collector
    fn moveable(&self) -> Self::BindData;

    /// Refill the bind collector with its bind data
    fn append_bind_data(&mut self, from: &Self::BindData);
}

#[derive(Debug)]
/// A bind collector used by backends which transmit bind parameters as an
/// opaque blob of bytes.
///
/// For most backends, this is the concrete implementation of `BindCollector`
/// that should be used.
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(metadata, binds)
)]
pub struct RawBytesBindCollector<DB: Backend + TypeMetadata> {
    /// The metadata associated with each bind parameter.
    ///
    /// This vec is guaranteed to be the same length as `binds`.
    pub(crate) metadata: Vec<DB::TypeMetadata>,
    /// The serialized bytes for each bind parameter.
    ///
    /// This vec is guaranteed to be the same length as `metadata`.
    pub(crate) binds: Vec<Option<Vec<u8>>>,
}

impl<DB: Backend + TypeMetadata> Default for RawBytesBindCollector<DB> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::new_without_default)]
impl<DB: Backend + TypeMetadata> RawBytesBindCollector<DB> {
    /// Construct an empty `RawBytesBindCollector`
    pub fn new() -> Self {
        RawBytesBindCollector {
            metadata: Vec::new(),
            binds: Vec::new(),
        }
    }

    pub(crate) fn reborrow_buffer<'a: 'b, 'b>(b: &'b mut ByteWrapper<'a>) -> ByteWrapper<'b> {
        ByteWrapper(b.0)
    }
}

impl<'a, DB> BindCollector<'a, DB> for RawBytesBindCollector<DB>
where
    for<'b> DB: Backend<BindCollector<'b> = Self> + TypeMetadata,
{
    type Buffer = ByteWrapper<'a>;

    fn push_bound_value<T, U>(
        &mut self,
        bind: &U,
        metadata_lookup: &mut DB::MetadataLookup,
    ) -> QueryResult<()>
    where
        DB: HasSqlType<T>,
        U: ToSql<T, DB> + ?Sized,
    {
        let mut bytes = Vec::new();
        let is_null = {
            let mut to_sql_output = Output::new(ByteWrapper(&mut bytes), metadata_lookup);
            bind.to_sql(&mut to_sql_output)
                .map_err(SerializationError)?
        };
        let metadata = <DB as HasSqlType<T>>::metadata(metadata_lookup);
        match is_null {
            IsNull::No => self.binds.push(Some(bytes)),
            IsNull::Yes => self.binds.push(None),
        }
        self.metadata.push(metadata);
        Ok(())
    }

    fn push_null_value(&mut self, metadata: DB::TypeMetadata) -> QueryResult<()> {
        self.metadata.push(metadata);
        self.binds.push(None);
        Ok(())
    }
}

impl<DB> MoveableBindCollector<DB> for RawBytesBindCollector<DB>
where
    for<'a> DB: Backend<BindCollector<'a> = Self> + TypeMetadata + 'static,
    <DB as TypeMetadata>::TypeMetadata: Clone + Send,
{
    type BindData = Self;

    fn moveable(&self) -> Self::BindData {
        RawBytesBindCollector {
            binds: self.binds.clone(),
            metadata: self.metadata.clone(),
        }
    }

    fn append_bind_data(&mut self, from: &Self::BindData) {
        self.binds.extend(from.binds.iter().cloned());
        self.metadata.extend(from.metadata.clone());
    }
}

// This is private for now as we may want to add `Into` impls for the wrapper type
// later on
mod private {
    /// A type wrapper for raw bytes
    #[derive(Debug)]
    pub struct ByteWrapper<'a>(pub(crate) &'a mut Vec<u8>);
}
