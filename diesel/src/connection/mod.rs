//! Types related to database connections

pub(crate) mod instrumentation;
#[cfg(all(
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub(crate) mod statement_cache;
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub mod statement_cache;
mod transaction_manager;

use crate::backend::Backend;
use crate::expression::QueryMetadata;
use crate::query_builder::{Query, QueryFragment, QueryId};
use crate::result::*;
use crate::sql_types::TypeMetadata;
use std::fmt::Debug;

#[doc(inline)]
pub use self::instrumentation::{
    get_default_instrumentation, set_default_instrumentation, DebugQuery, Instrumentation,
    InstrumentationEvent,
};
#[doc(inline)]
pub use self::transaction_manager::{
    AnsiTransactionManager, InTransactionStatus, TransactionDepthChange, TransactionManager,
    TransactionManagerStatus, ValidTransactionManagerStatus,
};

#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) use self::private::ConnectionSealed;

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::private::MultiConnectionHelper;

#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::instrumentation::StrQueryHelper;

#[cfg(all(
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
    any(feature = "sqlite", feature = "postgres", feature = "mysql")
))]
pub(crate) use self::private::MultiConnectionHelper;

/// Perform simple operations on a backend.
///
/// You should likely use [`Connection`] instead.
pub trait SimpleConnection {
    /// Execute multiple SQL statements within the same string.
    ///
    /// This function is used to execute migrations,
    /// which may contain more than one SQL statement.
    fn batch_execute(&mut self, query: &str) -> QueryResult<()>;
}

#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[deprecated(note = "Directly use `LoadConnection::Cursor` instead")]
pub type LoadRowIter<'conn, 'query, C, DB, B = DefaultLoadingMode> =
    <C as self::private::ConnectionHelperType<DB, B>>::Cursor<'conn, 'query>;

/// A connection to a database
///
/// This trait represents a database connection. It can be used to query the database through
/// the query dsl provided by diesel, custom extensions or raw sql queries.
///
/// # Implementing a custom connection
///
/// There are several reasons why you would want to implement a custom connection implementation:
///
/// * To wrap an existing connection for instrumentation purposes
/// * To use a different underlying library to provide a connection implementation
/// for already existing backends.
/// * To add support for an unsupported database system
///
/// Implementing a `Connection` in a third party crate requires
/// enabling the
/// `i-implement-a-third-party-backend-and-opt-into-breaking-changes`
/// crate feature which grants access to some of diesel's implementation details.
///
///
/// ## Wrapping an existing connection impl
///
/// Wrapping an existing connection allows you to customize the implementation to
/// add additional functionality, like for example instrumentation. For this use case
/// you only need to implement `Connection`, [`LoadConnection`] and all super traits.
/// You should forward any method call to the wrapped connection type.
/// It is **important** to also forward any method where diesel provides a
/// default implementation, as the wrapped connection implementation may
/// contain a customized implementation.
///
/// To allow the integration of your new connection type with other diesel features
#[cfg_attr(
    feature = "r2d2",
    doc = "it may be useful to also implement [`R2D2Connection`](crate::r2d2::R2D2Connection)"
)]
#[cfg_attr(
    not(feature = "r2d2"),
    doc = "it may be useful to also implement `R2D2Connection`"
)]
/// and [`MigrationConnection`](crate::migration::MigrationConnection).
///
/// ## Provide a new connection implementation for an existing backend
///
/// Implementing a new connection based on an existing backend can enable the usage of
/// other methods to connect to the database. One example here would be to replace
/// the official diesel provided connection implementations with an implementation
/// based on a pure rust connection crate.
///
/// **It's important to use prepared statements to implement the following methods:**
/// * [`LoadConnection::load`]
/// * [`Connection::execute_returning_count`]
///
/// For performance reasons it may also be meaningful to cache already prepared statements.
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    doc = "See [`StatementCache`](self::statement_cache::StatementCache)"
)]
#[cfg_attr(
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
    doc = "See `StatementCache`"
)]
/// for a helper type to implement prepared statement caching.
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    doc = "The [statement_cache](self::statement_cache)"
)]
#[cfg_attr(
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
    doc = "The statement_cache"
)]
/// module documentation contains details about efficient prepared statement caching
/// based on diesels query builder.
///
/// It is required to implement at least the following parts:
///
/// * A row type that describes how to receive values form a database row.
///   This type needs to implement [`Row`](crate::row::Row)
/// * A field type that describes a database field value.
///   This type needs to implement [`Field`](crate::row::Field)
/// * A connection type that wraps the connection +
///   the necessary state management.
/// * Maybe a [`TransactionManager`] implementation matching
///  the interface provided by the database connection crate.
///  Otherwise the implementation used by the corresponding
///  `Connection` in diesel can be reused.
///
/// To allow the integration of your new connection type with other diesel features
#[cfg_attr(
    feature = "r2d2",
    doc = "it may be useful to also implement [`R2D2Connection`](crate::r2d2::R2D2Connection)"
)]
#[cfg_attr(
    not(feature = "r2d2"),
    doc = "it may be useful to also implement `R2D2Connection`"
)]
/// and [`MigrationConnection`](crate::migration::MigrationConnection).
///
/// The exact implementation of the `Connection` trait depends on the interface provided
/// by the connection crate/library. A struct implementing `Connection` should
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    doc = "likely contain a [`StatementCache`](self::statement_cache::StatementCache)"
)]
#[cfg_attr(
    not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
    doc = "likely contain a `StatementCache`"
)]
/// to cache prepared statements efficiently.
///
/// As implementations differ significantly between the supported backends
/// we cannot give a one for all description here. Generally it's likely a
/// good idea to follow the implementation of the corresponding connection
/// in diesel at a high level to gain some idea how to implement your
/// custom implementation.
///
/// ## Implement support for an unsupported database system
///
/// Additionally to anything mentioned in the previous section the following steps are required:
///
/// * Implement a custom backend type. See the documentation of [`Backend`] for details
/// * Implement appropriate [`FromSql`](crate::deserialize::FromSql)/
/// [`ToSql`](crate::serialize::ToSql) conversions.
/// At least the following impls should be considered:
///     * `i16`: `FromSql<SmallInt, YourBackend>`
///     * `i32`: `FromSql<Integer, YourBackend>`
///     * `i64`: `FromSql<BigInt, YourBackend>`
///     * `f32`: `FromSql<Float, YourBackend>`
///     * `f64`: `FromSql<Double, YourBackend>`
///     * `bool`: `FromSql<Bool, YourBackend>`
///     * `String`: `FromSql<Text, YourBackend>`
///     * `Vec<u8>`: `FromSql<Binary, YourBackend>`
///     * `i16`: `ToSql<SmallInt, YourBackend>`
///     * `i32`: `ToSql<Integer, YourBackend>`
///     * `i64`: `ToSql<BigInt, YourBackend>`
///     * `f32`: `ToSql<Float, YourBackend>`
///     * `f64`: `ToSql<Double, YourBackend>`
///     * `bool`: `ToSql<Bool, YourBackend>`
///     * `String`: `ToSql<Text, YourBackend>`
///     * `Vec<u8>`: `ToSql<Binary, YourBackend>`
/// * Maybe a [`TransactionManager`] implementation matching
///  the interface provided by the database connection crate.
///  Otherwise the implementation used by the corresponding
///  `Connection` in diesel can be reused.
///
/// As these implementations will vary depending on the backend being used,
/// we cannot give concrete examples here. We recommend looking at our existing
/// implementations to see how you can implement your own connection.
pub trait Connection: SimpleConnection + Sized + Send
where
    // This trait bound is there so that implementing a new connection is
    // gated behind the `i-implement-a-third-party-backend-and-opt-into-breaking-changes`
    // feature flag
    Self: ConnectionSealed,
{
    /// The backend this type connects to
    type Backend: Backend;

    /// The transaction manager implementation used by this connection
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    type TransactionManager: TransactionManager<Self>;

    /// Establishes a new connection to the database
    ///
    /// The argument to this method and the method's behavior varies by backend.
    /// See the documentation for that backend's connection class
    /// for details about what it accepts and how it behaves.
    fn establish(database_url: &str) -> ConnectionResult<Self>;

    /// Executes the given function inside of a database transaction
    ///
    /// This function executes the provided closure `f` inside a database
    /// transaction. If there is already an open transaction for the current
    /// connection savepoints will be used instead. The connection is committed if
    /// the closure returns `Ok(_)`, it will be rolled back if it returns `Err(_)`.
    /// For both cases the original result value will be returned from this function.
    ///
    /// If the transaction fails to commit due to a `SerializationFailure` or a
    /// `ReadOnlyTransaction` a rollback will be attempted.
    /// If the rollback fails, the error will be returned in a
    /// [`Error::RollbackErrorOnCommit`],
    /// from which you will be able to extract both the original commit error and
    /// the rollback error.
    /// In addition, the connection will be considered broken
    /// as it contains a uncommitted unabortable open transaction. Any further
    /// interaction with the transaction system will result in an returned error
    /// in this case.
    ///
    /// If the closure returns an `Err(_)` and the rollback fails the function
    /// will return that rollback error directly, and the transaction manager will
    /// be marked as broken as it contains a uncommitted unabortable open transaction.
    ///
    /// If a nested transaction fails to release the corresponding savepoint
    /// the error will be returned directly.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    ///
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// conn.transaction::<_, Error, _>(|conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// })?;
    ///
    /// conn.transaction::<(), _, _>(|conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Pascal"))
    ///         .execute(conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby", "Pascal"], all_names);
    ///
    ///     // If we want to roll back the transaction, but don't have an
    ///     // actual error to return, we can return `RollbackTransaction`.
    ///     Err(Error::RollbackTransaction)
    /// });
    ///
    /// let all_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: From<Error>,
    {
        Self::TransactionManager::transaction(self, f)
    }

    /// Creates a transaction that will never be committed. This is useful for
    /// tests. Panics if called while inside of a transaction or
    /// if called with a connection containing a broken transaction
    fn begin_test_transaction(&mut self) -> QueryResult<()> {
        match Self::TransactionManager::transaction_manager_status_mut(self) {
            TransactionManagerStatus::Valid(valid_status) => {
                assert_eq!(None, valid_status.transaction_depth())
            }
            TransactionManagerStatus::InError => panic!("Transaction manager in error"),
        };
        Self::TransactionManager::begin_transaction(self)?;
        // set the test transaction flag
        // to prevent that this connection gets dropped in connection pools
        // Tests commonly set the poolsize to 1 and use `begin_test_transaction`
        // to prevent modifications to the schema
        Self::TransactionManager::transaction_manager_status_mut(self).set_test_transaction_flag();
        Ok(())
    }

    /// Executes the given function inside a transaction, but does not commit
    /// it. Panics if the given function returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::result::Error;
    ///
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// conn.test_transaction::<_, Error, _>(|conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn)?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// });
    ///
    /// // Even though we returned `Ok`, the transaction wasn't committed.
    /// let all_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(vec!["Sean", "Tess"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn test_transaction<T, E, F>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
        E: Debug,
    {
        let mut user_result = None;
        let _ = self.transaction::<(), _, _>(|conn| {
            user_result = f(conn).ok();
            Err(Error::RollbackTransaction)
        });
        user_result.expect("Transaction did not succeed")
    }

    /// Execute a single SQL statements given by a query and return
    /// number of affected rows
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId;

    /// Get access to the current transaction state of this connection
    ///
    /// This function should be used from [`TransactionManager`] to access
    /// internally required state.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData;

    /// Get the instrumentation instance stored in this connection
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn instrumentation(&mut self) -> &mut dyn Instrumentation;

    /// Set a specific [`Instrumentation`] implementation for this connection
    fn set_instrumentation(&mut self, instrumentation: impl Instrumentation);
}

/// The specific part of a [`Connection`] which actually loads data from the database
///
/// This is a separate trait to allow connection implementations to specify
/// different loading modes via the generic parameter.
pub trait LoadConnection<B = DefaultLoadingMode>: Connection {
    /// The cursor type returned by [`LoadConnection::load`]
    ///
    /// Users should handle this as opaque type that implements [`Iterator`]
    type Cursor<'conn, 'query>: Iterator<
        Item = QueryResult<<Self as LoadConnection<B>>::Row<'conn, 'query>>,
    >
    where
        Self: 'conn;

    /// The row type used as [`Iterator::Item`] for the iterator implementation
    /// of [`LoadConnection::Cursor`]
    type Row<'conn, 'query>: crate::row::Row<'conn, Self::Backend>
    where
        Self: 'conn;

    /// Executes a given query and returns any requested values
    ///
    /// This function executes a given query and returns the
    /// query result as given by the database. **Normal users
    /// should not use this function**. Use
    /// [`QueryDsl::load`](crate::QueryDsl) instead.
    ///
    /// This function is useful for people trying to build an alternative
    /// dsl on top of diesel. It returns an [`impl Iterator<Item = QueryResult<&impl Row<Self::Backend>>`](Iterator).
    /// This type can be used to iterate over all rows returned by the database.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>;
}

/// Describes a connection with an underlying [`crate::sql_types::TypeMetadata::MetadataLookup`]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub trait WithMetadataLookup: Connection {
    /// Retrieves the underlying metadata lookup
    fn metadata_lookup(&mut self) -> &mut <Self::Backend as TypeMetadata>::MetadataLookup;
}

/// A variant of the [`Connection`](trait.Connection.html) trait that is
/// usable with dynamic dispatch
///
/// If you are looking for a way to use pass database connections
/// for different database backends around in your application
/// this trait won't help you much. Normally you should only
/// need to use this trait if you are interacting with a connection
/// passed to a [`Migration`](../migration/trait.Migration.html)
pub trait BoxableConnection<DB: Backend>: SimpleConnection + std::any::Any {
    /// Maps the current connection to `std::any::Any`
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn as_any(&self) -> &dyn std::any::Any;

    #[doc(hidden)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<C> BoxableConnection<C::Backend> for C
where
    C: Connection + std::any::Any,
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The default loading mode provided by a [`Connection`].
///
/// Checkout the documentation of concrete connection types for details about
/// supported loading modes.
///
/// All types implementing [`Connection`] should provide at least
/// a single [`LoadConnection<DefaultLoadingMode>`](self::LoadConnection)
/// implementation.
#[derive(Debug, Copy, Clone)]
pub struct DefaultLoadingMode;

impl<DB: Backend + 'static> dyn BoxableConnection<DB> {
    /// Downcast the current connection to a specific connection
    /// type.
    ///
    /// This will return `None` if the underlying
    /// connection does not match the corresponding
    /// type, otherwise a reference to the underlying connection is returned
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Connection<Backend = DB> + 'static,
    {
        self.as_any().downcast_ref::<T>()
    }

    /// Downcast the current connection to a specific mutable connection
    /// type.
    ///
    /// This will return `None` if the underlying
    /// connection does not match the corresponding
    /// type, otherwise a mutable reference to the underlying connection is returned
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Connection<Backend = DB> + 'static,
    {
        self.as_any_mut().downcast_mut::<T>()
    }

    /// Check if the current connection is
    /// a specific connection type
    pub fn is<T>(&self) -> bool
    where
        T: Connection<Backend = DB> + 'static,
    {
        self.as_any().is::<T>()
    }
}

// These traits are considered private for different reasons:
//
// `ConnectionSealed` to control who can implement `Connection`,
// so that we can later change the `Connection` trait
//
// `MultiConnectionHelper` is a workaround needed for the
// `MultiConnection` derive. We might stabilize this trait with
// the corresponding derive
//
// `ConnectionHelperType` as a workaround for the `LoadRowIter`
// type def. That trait should not be used by any user outside of diesel,
// it purely exists for backward compatibility reasons.
pub(crate) mod private {

    /// This trait restricts who can implement `Connection`
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub trait ConnectionSealed {}

    /// This trait provides helper methods to convert a database lookup type
    /// to/from an `std::any::Any` reference. This is used internally by the `#[derive(MultiConnection)]`
    /// implementation
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub trait MultiConnectionHelper: super::Connection {
        /// Convert the lookup type to any
        fn to_any<'a>(
            lookup: &mut <Self::Backend as crate::sql_types::TypeMetadata>::MetadataLookup,
        ) -> &mut (dyn std::any::Any + 'a);

        /// Get the lookup type from any
        fn from_any(
            lookup: &mut dyn std::any::Any,
        ) -> Option<&mut <Self::Backend as crate::sql_types::TypeMetadata>::MetadataLookup>;
    }

    // These impls are only there for backward compatibility reasons
    // Remove them on the next breaking release
    #[allow(unreachable_pub)] // must be pub for the type def using this trait
    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    pub trait ConnectionHelperType<DB, B>: super::LoadConnection<B, Backend = DB> {
        type Cursor<'conn, 'query>
        where
            Self: 'conn;
    }
    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    impl<T, B> ConnectionHelperType<T::Backend, B> for T
    where
        T: super::LoadConnection<B>,
    {
        type Cursor<'conn, 'query> = <T as super::LoadConnection<B>>::Cursor<'conn, 'query> where T: 'conn;
    }
}
