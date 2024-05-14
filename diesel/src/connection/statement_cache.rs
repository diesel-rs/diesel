//! Helper types for prepared statement caching
//!
//! A primer on prepared statement caching in Diesel
//! ------------------------------------------------
//!
//! Diesel uses prepared statements for virtually all queries. This is most
//! visible in our lack of any sort of "quoting" API. Values must always be
//! transmitted as bind parameters, we do not support direct interpolation. The
//! only method in the public API that doesn't require the use of prepared
//! statements is [`SimpleConnection::batch_execute`](super::SimpleConnection::batch_execute).
//!
//! In order to avoid the cost of re-parsing and planning subsequent queries,
//! Diesel caches the prepared statement whenever possible. Queries will fall
//! into one of three buckets:
//!
//! - Unsafe to cache
//! - Cached by SQL
//! - Cached by type
//!
//! A query is considered unsafe to cache if it represents a potentially
//! unbounded number of queries. This is communicated to the connection through
//! [`QueryFragment::is_safe_to_cache_prepared`]. While this is done as a full AST
//! pass, after monomorphisation and inlining this will usually be optimized to
//! a constant. Only boxed queries will need to do actual work to answer this
//! question.
//!
//! The majority of AST nodes are safe to cache if their components are safe to
//! cache. There are at least 4 cases where a query is unsafe to cache:
//!
//! - queries containing `IN` with bind parameters
//!     - This requires 1 bind parameter per value, and is therefore unbounded
//!     - `IN` with subselects are cached (assuming the subselect is safe to
//!        cache)
//!     - `IN` statements for postgresql are cached as they use `= ANY($1)` instead
//!        which does not cause a unbound number of binds
//! - `INSERT` statements with a variable number of rows
//!     - The SQL varies based on the number of rows being inserted.
//! - `UPDATE` statements
//!     - Technically it's bounded on "number of optional values being passed to
//!       `SET` factorial" but that's still quite high, and not worth caching
//!       for the same reason as single row inserts
//! - `SqlLiteral` nodes
//!     - We have no way of knowing whether the SQL was generated dynamically or
//!       not, so we must assume that it's unbounded
//!
//! For queries which are unsafe to cache, the statement cache will never insert
//! them. They will be prepared and immediately released after use (or in the
//! case of PG they will use the unnamed prepared statement).
//!
//! For statements which are able to be cached, we then have to determine what
//! to use as the cache key. The standard method that virtually all ORMs or
//! database access layers use in the wild is to store the statements in a
//! hash map, using the SQL as the key.
//!
//! However, the majority of queries using Diesel that are safe to cache as
//! prepared statements will be uniquely identified by their type. For these
//! queries, we can bypass the query builder entirely. Since our AST is
//! generally optimized away by the compiler, for these queries the cost of
//! fetching a prepared statement from the cache is the cost of [`HashMap<u32,
//! _>::get`], where the key we're fetching by is a compile time constant. For
//! these types, the AST pass to gather the bind parameters will also be
//! optimized to accessing each parameter individually.
//!
//! Determining if a query can be cached by type is the responsibility of the
//! [`QueryId`] trait. This trait is quite similar to `Any`, but with a few
//! differences:
//!
//! - No `'static` bound
//!     - Something being a reference never changes the SQL that is generated,
//!       so `&T` has the same query id as `T`.
//! - `Option<TypeId>` instead of `TypeId`
//!     - We need to be able to constrain on this trait being implemented, but
//!       not all types will actually have a static query id. Hopefully once
//!       specialization is stable we can remove the `QueryId` bound and
//!       specialize on it instead (or provide a blanket impl for all `T`)
//! - Implementors give a more broad type than `Self`
//!     - This really only affects bind parameters. There are 6 different Rust
//!       types which can be used for a parameter of type `timestamp`. The same
//!       statement can be used regardless of the Rust type, so [`Bound<ST, T>`](crate::expression::bound::Bound)
//!       defines its [`QueryId`] as [`Bound<ST, ()>`](crate::expression::bound::Bound).
//!
//! A type returning `Some(id)` or `None` for its query ID is based on whether
//! the SQL it generates can change without the type changing. At the moment,
//! the only type which is safe to cache as a prepared statement but does not
//! have a static query ID is something which has been boxed.
//!
//! One potential optimization that we don't perform is storing the queries
//! which are cached by type ID in a separate map. Since a type ID is a u64,
//! this would allow us to use a specialized map which knows that there will
//! never be hashing collisions (also known as a perfect hashing function),
//! which would mean lookups are always constant time. However, this would save
//! nanoseconds on an operation that will take microseconds or even
//! milliseconds.

use std::any::TypeId;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use crate::backend::Backend;
use crate::connection::InstrumentationEvent;
use crate::query_builder::*;
use crate::result::QueryResult;

use super::Instrumentation;

/// A prepared statement cache
#[allow(missing_debug_implementations, unreachable_pub)]
#[cfg_attr(
    docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
pub struct StatementCache<DB: Backend, Statement> {
    pub(crate) cache: HashMap<StatementCacheKey<DB>, Statement>,
}

/// A helper type that indicates if a certain query
/// is cached inside of the prepared statement cache or not
///
/// This information can be used by the connection implementation
/// to signal this fact to the database while actually
/// preparing the statement
#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
#[allow(unreachable_pub)]
pub enum PrepareForCache {
    /// The statement will be cached
    Yes,
    /// The statement won't be cached
    No,
}

#[allow(
    clippy::len_without_is_empty,
    clippy::new_without_default,
    unreachable_pub
)]
impl<DB, Statement> StatementCache<DB, Statement>
where
    DB: Backend,
    DB::TypeMetadata: Clone,
    DB::QueryBuilder: Default,
    StatementCacheKey<DB>: Hash + Eq,
{
    /// Create a new prepared statement cache
    #[allow(unreachable_pub)]
    pub fn new() -> Self {
        StatementCache {
            cache: HashMap::new(),
        }
    }

    /// Get the current length of the statement cache
    #[allow(unreachable_pub)]
    #[cfg(any(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        feature = "postgres",
        all(feature = "sqlite", test)
    ))]
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Prepare a query as prepared statement
    ///
    /// This functions returns a prepared statement corresponding to the
    /// query passed as `source` with the bind values passed as `bind_types`.
    /// If the query is already cached inside this prepared statement cache
    /// the cached prepared statement will be returned, otherwise `prepare_fn`
    /// will be called to create a new prepared statement for this query source.
    /// The first parameter of the callback contains the query string, the second
    /// parameter indicates if the constructed prepared statement will be cached or not.
    /// See the [module](self) documentation for details
    /// about which statements are cached and which are not cached.
    #[allow(unreachable_pub)]
    pub fn cached_statement<T, F>(
        &mut self,
        source: &T,
        backend: &DB,
        bind_types: &[DB::TypeMetadata],
        mut prepare_fn: F,
        instrumentation: &mut dyn Instrumentation,
    ) -> QueryResult<MaybeCached<'_, Statement>>
    where
        T: QueryFragment<DB> + QueryId,
        F: FnMut(&str, PrepareForCache) -> QueryResult<Statement>,
    {
        self.cached_statement_non_generic(
            T::query_id(),
            source,
            backend,
            bind_types,
            &mut prepare_fn,
            instrumentation,
        )
    }

    /// Reduce the amount of monomorphized code by factoring this via dynamic dispatch
    fn cached_statement_non_generic(
        &mut self,
        maybe_type_id: Option<TypeId>,
        source: &dyn QueryFragmentForCachedStatement<DB>,
        backend: &DB,
        bind_types: &[DB::TypeMetadata],
        prepare_fn: &mut dyn FnMut(&str, PrepareForCache) -> QueryResult<Statement>,
        instrumentation: &mut dyn Instrumentation,
    ) -> QueryResult<MaybeCached<'_, Statement>> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        let cache_key = StatementCacheKey::for_source(maybe_type_id, source, bind_types, backend)?;

        if !source.is_safe_to_cache_prepared(backend)? {
            let sql = cache_key.sql(source, backend)?;
            return prepare_fn(&sql, PrepareForCache::No).map(MaybeCached::CannotCache);
        }

        let cached_result = match self.cache.entry(cache_key) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let statement = {
                    let sql = entry.key().sql(source, backend)?;
                    instrumentation
                        .on_connection_event(InstrumentationEvent::CacheQuery { sql: &sql });
                    prepare_fn(&sql, PrepareForCache::Yes)
                };

                entry.insert(statement?)
            }
        };

        Ok(MaybeCached::Cached(cached_result))
    }
}

/// Implemented for all `QueryFragment`s, dedicated to dynamic dispatch within the context of
/// `statement_cache`
///
/// We want the generated code to be as small as possible, so for each query passed to
/// [`StatementCache::cached_statement`] the generated assembly will just call a non generic
/// version with dynamic dispatch pointing to the VTABLE of this minimal trait
///
/// This preserves the opportunity for the compiler to entirely optimize the `construct_sql`
/// function as a function that simply returns a constant `String`.
#[allow(unreachable_pub)]
#[cfg_attr(
    docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
pub trait QueryFragmentForCachedStatement<DB> {
    /// Convert the query fragment into a SQL string for the given backend
    fn construct_sql(&self, backend: &DB) -> QueryResult<String>;
    /// Check whether it's safe to cache the query
    fn is_safe_to_cache_prepared(&self, backend: &DB) -> QueryResult<bool>;
}
impl<T, DB> QueryFragmentForCachedStatement<DB> for T
where
    DB: Backend,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn construct_sql(&self, backend: &DB) -> QueryResult<String> {
        let mut query_builder = DB::QueryBuilder::default();
        self.to_sql(&mut query_builder, backend)?;
        Ok(query_builder.finish())
    }

    fn is_safe_to_cache_prepared(&self, backend: &DB) -> QueryResult<bool> {
        <T as QueryFragment<DB>>::is_safe_to_cache_prepared(self, backend)
    }
}

/// Wraps a possibly cached prepared statement
///
/// Essentially a customized version of [`Cow`]
/// that does not depend on [`ToOwned`]
#[allow(missing_debug_implementations, unreachable_pub)]
#[cfg_attr(
    docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
#[non_exhaustive]
pub enum MaybeCached<'a, T: 'a> {
    /// Contains a not cached prepared statement
    CannotCache(T),
    /// Contains a reference cached prepared statement
    Cached(&'a mut T),
}

impl<'a, T> Deref for MaybeCached<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match *self {
            MaybeCached::CannotCache(ref x) => x,
            MaybeCached::Cached(ref x) => x,
        }
    }
}

impl<'a, T> DerefMut for MaybeCached<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match *self {
            MaybeCached::CannotCache(ref mut x) => x,
            MaybeCached::Cached(ref mut x) => x,
        }
    }
}

/// The lookup key used by [`StatementCache`] internally
///
/// This can contain either a at compile time known type id
/// (representing a statically known query) or a at runtime
/// calculated query string + parameter types (for queries
/// that may change depending on their parameters)
#[allow(missing_debug_implementations, unreachable_pub)]
#[derive(Hash, PartialEq, Eq)]
#[cfg_attr(
    docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
pub enum StatementCacheKey<DB: Backend> {
    /// Represents a at compile time known query
    ///
    /// Calculated via [`QueryId::QueryId`]
    Type(TypeId),
    /// Represents a dynamically constructed query
    ///
    /// This variant is used if [`QueryId::HAS_STATIC_QUERY_ID`]
    /// is `false` and [`AstPass::unsafe_to_cache_prepared`] is not
    /// called for a given query.
    Sql {
        /// contains the sql query string
        sql: String,
        /// contains the types of any bind parameter passed to the query
        bind_types: Vec<DB::TypeMetadata>,
    },
}

impl<DB> StatementCacheKey<DB>
where
    DB: Backend,
    DB::QueryBuilder: Default,
    DB::TypeMetadata: Clone,
{
    /// Create a new statement cache key for the given query source
    // Note: Intentionally monomorphic over source.
    #[allow(unreachable_pub)]
    pub fn for_source(
        maybe_type_id: Option<TypeId>,
        source: &dyn QueryFragmentForCachedStatement<DB>,
        bind_types: &[DB::TypeMetadata],
        backend: &DB,
    ) -> QueryResult<Self> {
        match maybe_type_id {
            Some(id) => Ok(StatementCacheKey::Type(id)),
            None => {
                let sql = source.construct_sql(backend)?;
                Ok(StatementCacheKey::Sql {
                    sql,
                    bind_types: bind_types.into(),
                })
            }
        }
    }

    /// Get the sql for a given query source based
    ///
    /// This is an optimization that may skip constructing the query string
    /// twice if it's already part of the current cache key
    // Note: Intentionally monomorphic over source.
    #[allow(unreachable_pub)]
    pub fn sql(
        &self,
        source: &dyn QueryFragmentForCachedStatement<DB>,
        backend: &DB,
    ) -> QueryResult<Cow<'_, str>> {
        match *self {
            StatementCacheKey::Type(_) => source.construct_sql(backend).map(Cow::Owned),
            StatementCacheKey::Sql { ref sql, .. } => Ok(Cow::Borrowed(sql)),
        }
    }
}
