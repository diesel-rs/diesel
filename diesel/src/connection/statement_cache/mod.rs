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
//! by default Diesel caches the prepared statement whenever possible. This
//! can be customized by calling
//! [`Connection::set_cache_size`](super::Connection::set_prepared_statement_cache_size).
//!
//! Queries will fall into one of three buckets:
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
//!       cache)
//!     - `IN` statements for postgresql are cached as they use `= ANY($1)` instead
//!       which does not cause an unbound number of binds
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
//! _>::get`](std::collections::HashMap::get), where the key we're fetching by is a compile time constant. For
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
use std::collections::hash_map::Entry;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use strategy::{
    LookupStatementResult, StatementCacheStrategy, WithCacheStrategy, WithoutCacheStrategy,
};

use crate::backend::Backend;
use crate::connection::InstrumentationEvent;
use crate::query_builder::*;
use crate::result::QueryResult;

use super::{CacheSize, Instrumentation};

/// Various interfaces and implementations to control connection statement caching.
#[allow(unreachable_pub)]
pub mod strategy;

/// A prepared statement cache
#[allow(missing_debug_implementations, unreachable_pub)]
#[cfg_attr(
    diesel_docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
pub struct StatementCache<DB: Backend, Statement> {
    cache: Box<dyn StatementCacheStrategy<DB, Statement>>,
    // increment every time a query is cached
    // some backends might use it to create unique prepared statement names
    cache_counter: u64,
}

/// A helper type that indicates if a certain query
/// is cached inside of the prepared statement cache or not
///
/// This information can be used by the connection implementation
/// to signal this fact to the database while actually
/// preparing the statement
#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    diesel_docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
#[allow(unreachable_pub)]
pub enum PrepareForCache {
    /// The statement will be cached
    Yes {
        /// Counter might be used as unique identifier for prepared statement.
        #[allow(dead_code)]
        counter: u64,
    },
    /// The statement won't be cached
    No,
}

#[allow(clippy::new_without_default, unreachable_pub)]
impl<DB, Statement> StatementCache<DB, Statement>
where
    DB: Backend + 'static,
    Statement: Send + 'static,
    DB::TypeMetadata: Send + Clone,
    DB::QueryBuilder: Default,
    StatementCacheKey<DB>: Hash + Eq,
{
    /// Create a new prepared statement cache using [`CacheSize::Unbounded`] as caching strategy.
    #[allow(unreachable_pub)]
    pub fn new() -> Self {
        StatementCache {
            cache: Box::new(WithCacheStrategy::default()),
            cache_counter: 0,
        }
    }

    /// Set caching strategy from predefined implementations
    pub fn set_cache_size(&mut self, size: CacheSize) {
        if self.cache.cache_size() != size {
            self.cache = match size {
                CacheSize::Unbounded => Box::new(WithCacheStrategy::default()),
                CacheSize::Disabled => Box::new(WithoutCacheStrategy::default()),
            }
        }
    }

    /// Setting custom caching strategy. It is used in tests, to verify caching logic
    #[allow(dead_code)]
    pub(crate) fn set_strategy<Strategy>(&mut self, s: Strategy)
    where
        Strategy: StatementCacheStrategy<DB, Statement> + 'static,
    {
        self.cache = Box::new(s);
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
    //
    // Notes:
    // This function takes explicitly a connection and a function pointer (and no generic callback)
    // as argument to ensure that we don't leak generic query types into the prepare function
    #[allow(unreachable_pub)]
    #[cfg(any(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        feature = "sqlite",
        feature = "mysql"
    ))]
    pub fn cached_statement<'a, T, R, C>(
        &'a mut self,
        source: &T,
        backend: &DB,
        bind_types: &[DB::TypeMetadata],
        conn: C,
        prepare_fn: fn(C, &str, PrepareForCache, &[DB::TypeMetadata]) -> R,
        instrumentation: &mut dyn Instrumentation,
    ) -> R::Return<'a>
    where
        T: QueryFragment<DB> + QueryId,
        R: StatementCallbackReturnType<Statement, C> + 'a,
    {
        self.cached_statement_non_generic(
            T::query_id(),
            source,
            backend,
            bind_types,
            conn,
            prepare_fn,
            instrumentation,
        )
    }

    /// Prepare a query as prepared statement
    ///
    /// This function closely mirrors `Self::cached_statement` but
    /// eliminates the generic query type in favour of a trait object
    ///
    /// This can be easier to use in situations where you already turned
    /// the query type into a concrete SQL string
    // Notes:
    // This function takes explicitly a connection and a function pointer (and no generic callback)
    // as argument to ensure that we don't leak generic query types into the prepare function
    #[allow(unreachable_pub)]
    #[allow(clippy::too_many_arguments)] // we need all of them
    pub fn cached_statement_non_generic<'a, R, C>(
        &'a mut self,
        maybe_type_id: Option<TypeId>,
        source: &dyn QueryFragmentForCachedStatement<DB>,
        backend: &DB,
        bind_types: &[DB::TypeMetadata],
        conn: C,
        prepare_fn: fn(C, &str, PrepareForCache, &[DB::TypeMetadata]) -> R,
        instrumentation: &mut dyn Instrumentation,
    ) -> R::Return<'a>
    where
        R: StatementCallbackReturnType<Statement, C> + 'a,
    {
        Self::cached_statement_non_generic_impl(
            self.cache.as_mut(),
            maybe_type_id,
            source,
            backend,
            bind_types,
            conn,
            |conn, sql, is_cached| {
                if is_cached {
                    instrumentation.on_connection_event(InstrumentationEvent::CacheQuery { sql });
                    self.cache_counter += 1;
                    prepare_fn(
                        conn,
                        sql,
                        PrepareForCache::Yes {
                            counter: self.cache_counter,
                        },
                        bind_types,
                    )
                } else {
                    prepare_fn(conn, sql, PrepareForCache::No, bind_types)
                }
            },
        )
    }

    /// Reduce the amount of monomorphized code by factoring this via dynamic dispatch
    /// There will be only one instance of `R` for diesel (and a different single instance for diesel-async)
    /// There will be only a instance per connection type `C` for each connection that
    /// uses this prepared statement impl, this closely correlates to the types `DB` and `Statement`
    /// for the overall statement cache impl
    fn cached_statement_non_generic_impl<'a, R, C>(
        cache: &'a mut dyn StatementCacheStrategy<DB, Statement>,
        maybe_type_id: Option<TypeId>,
        source: &dyn QueryFragmentForCachedStatement<DB>,
        backend: &DB,
        bind_types: &[DB::TypeMetadata],
        conn: C,
        prepare_fn: impl FnOnce(C, &str, bool) -> R,
    ) -> R::Return<'a>
    where
        R: StatementCallbackReturnType<Statement, C> + 'a,
    {
        // this function cannot use the `?` operator
        // as we want to abstract over returning `QueryResult<MaybeCached>` and
        // `impl Future<Output = QueryResult<MaybeCached>>` here
        // to share the prepared statement cache implementation between diesel and
        // diesel_async
        //
        // For this reason we need to match explicitly on each error and call `R::from_error()`
        // to construct the right error return variant
        let cache_key =
            match StatementCacheKey::for_source(maybe_type_id, source, bind_types, backend) {
                Ok(o) => o,
                Err(e) => return R::from_error(e),
            };
        let is_safe_to_cache_prepared = match source.is_safe_to_cache_prepared(backend) {
            Ok(o) => o,
            Err(e) => return R::from_error(e),
        };
        // early return if the statement cannot be cached
        if !is_safe_to_cache_prepared {
            let sql = match cache_key.sql(source, backend) {
                Ok(sql) => sql,
                Err(e) => return R::from_error(e),
            };
            return prepare_fn(conn, &sql, false).map_to_no_cache();
        }
        let entry = cache.lookup_statement(cache_key);
        match entry {
            // The statement is already cached
            LookupStatementResult::CacheEntry(Entry::Occupied(e)) => {
                R::map_to_cache(e.into_mut(), conn)
            }
            // The statement is not cached but there is capacity to cache it
            LookupStatementResult::CacheEntry(Entry::Vacant(e)) => {
                let sql = match e.key().sql(source, backend) {
                    Ok(sql) => sql,
                    Err(e) => return R::from_error(e),
                };
                let st = prepare_fn(conn, &sql, true);
                st.register_cache(|stmt| e.insert(stmt))
            }
            // The statement is not cached and there is no capacity to cache it
            LookupStatementResult::NoCache(cache_key) => {
                let sql = match cache_key.sql(source, backend) {
                    Ok(sql) => sql,
                    Err(e) => return R::from_error(e),
                };
                prepare_fn(conn, &sql, false).map_to_no_cache()
            }
        }
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
    diesel_docsrs,
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
    diesel_docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
#[non_exhaustive]
pub enum MaybeCached<'a, T: 'a> {
    /// Contains a not cached prepared statement
    CannotCache(T),
    /// Contains a reference cached prepared statement
    Cached(&'a mut T),
}

/// This trait abstracts over the type returned by the prepare statement function
///
/// The main use-case for this abstraction is to share the same statement cache implementation
/// between diesel and diesel-async.
#[cfg_attr(
    diesel_docsrs,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
#[allow(unreachable_pub)]
pub trait StatementCallbackReturnType<S: 'static, C> {
    /// The return type of `StatementCache::cached_statement`
    ///
    /// Either a `QueryResult<MaybeCached<S>>` or a future of that result type
    type Return<'a>;

    /// Create the return type from an error
    fn from_error<'a>(e: diesel::result::Error) -> Self::Return<'a>;

    /// Map the callback return type to the `MaybeCached::CannotCache` variant
    fn map_to_no_cache<'a>(self) -> Self::Return<'a>
    where
        Self: 'a;

    /// Map the cached statement to the `MaybeCached::Cached` variant
    fn map_to_cache(stmt: &mut S, conn: C) -> Self::Return<'_>;

    /// Insert the created statement into the cache via the provided callback
    /// and then turn the returned reference into `MaybeCached::Cached`
    fn register_cache<'a>(
        self,
        callback: impl FnOnce(S) -> &'a mut S + Send + 'a,
    ) -> Self::Return<'a>
    where
        Self: 'a;
}

impl<S, C> StatementCallbackReturnType<S, C> for QueryResult<S>
where
    S: 'static,
{
    type Return<'a> = QueryResult<MaybeCached<'a, S>>;

    fn from_error<'a>(e: diesel::result::Error) -> Self::Return<'a> {
        Err(e)
    }

    fn map_to_no_cache<'a>(self) -> Self::Return<'a> {
        self.map(MaybeCached::CannotCache)
    }

    fn map_to_cache(stmt: &mut S, _conn: C) -> Self::Return<'_> {
        Ok(MaybeCached::Cached(stmt))
    }

    fn register_cache<'a>(
        self,
        callback: impl FnOnce(S) -> &'a mut S + Send + 'a,
    ) -> Self::Return<'a>
    where
        Self: 'a,
    {
        Ok(MaybeCached::Cached(callback(self?)))
    }
}

impl<T> Deref for MaybeCached<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match *self {
            MaybeCached::CannotCache(ref x) => x,
            MaybeCached::Cached(ref x) => x,
        }
    }
}

impl<T> DerefMut for MaybeCached<'_, T> {
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
    diesel_docsrs,
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
