use downcast_rs::Downcast;
use std::fmt::{Debug, Display};
use std::num::NonZeroU32;
use std::ops::{Deref, DerefMut};

static GLOBAL_INSTRUMENTATION: std::sync::RwLock<fn() -> Option<Box<dyn Instrumentation>>> =
    std::sync::RwLock::new(|| None);

/// A helper trait for opaque query representations
/// which allows to get a `Display` and `Debug`
/// representation of the underlying type without
/// exposing type specific details
pub trait DebugQuery: Debug + Display {}

impl<T, DB> DebugQuery for crate::query_builder::DebugQuery<'_, T, DB> where Self: Debug + Display {}

/// A helper type that allows printing out str slices
///
/// This type is necessary because it's not possible
/// to cast from a reference of a unsized type like `&str`
/// to a reference of a trait object even if that
/// type implements all necessary traits
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) struct StrQueryHelper<'query> {
    s: &'query str,
}

impl<'query> StrQueryHelper<'query> {
    /// Construct a new `StrQueryHelper`
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    #[cfg(any(
        feature = "postgres",
        feature = "sqlite",
        feature = "mysql",
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    ))]
    pub(crate) fn new(s: &'query str) -> Self {
        Self { s }
    }
}

impl Debug for StrQueryHelper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.s, f)
    }
}

impl Display for StrQueryHelper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.s, f)
    }
}

impl DebugQuery for StrQueryHelper<'_> {}

/// This enum describes possible connection events
/// that can be handled by an [`Instrumentation`] implementation
///
/// Some fields might contain sensitive information, like login
/// details for the database.
///
/// Diesel does not guarantee that future versions will
/// emit the same events in the same order or timing.
/// In addition the output of the [`Debug`] and [`Display`]
/// implementation of the enum itself and any of its fields
/// is not guarantee to be stable.
//
// This type is carefully designed
// to avoid any potential overhead by
// taking references for all things
// and by not performing any additional
// work until required.
// In addition it's carefully designed
// not to be dependent on the actual backend
// type, as that makes it easier to reuse
// `Instrumentation` implementations in
// a different context
#[derive(Debug)]
#[non_exhaustive]
pub enum InstrumentationEvent<'a> {
    /// An event emitted by before starting
    /// establishing a new connection
    #[non_exhaustive]
    StartEstablishConnection {
        /// The database url the connection
        /// tries to connect to
        ///
        /// This might contain sensitive information
        /// like the database password
        url: &'a str,
    },
    /// An event emitted after establishing a
    /// new connection
    #[non_exhaustive]
    FinishEstablishConnection {
        /// The database url the connection
        /// tries is connected to
        ///
        /// This might contain sensitive information
        /// like the database password
        url: &'a str,
        /// An optional error if the connection failed
        error: Option<&'a crate::result::ConnectionError>,
    },
    /// An event that is emitted before executing
    /// a query
    #[non_exhaustive]
    StartQuery {
        /// A opaque representation of the query
        ///
        /// This type implements [`Debug`] and [`Display`],
        /// but should be considered otherwise as opaque.
        ///
        /// The exact output of the [`Debug`] and [`Display`]
        /// implementation is not considered as part of the
        /// stable API.
        query: &'a dyn DebugQuery,
    },
    /// An event that is emitted when a query
    /// is cached in the connection internal
    /// prepared statement cache
    #[non_exhaustive]
    CacheQuery {
        /// SQL string of the cached query
        sql: &'a str,
    },
    /// An event that is emitted after executing
    /// a query
    #[non_exhaustive]
    FinishQuery {
        /// A opaque representation of the query
        ///
        /// This type implements [`Debug`] and [`Display`],
        /// but should be considered otherwise as opaque.
        ///
        /// The exact output of the [`Debug`] and [`Display`]
        /// implementation is not considered as part of the
        /// stable API.
        query: &'a dyn DebugQuery,
        /// An optional error if the connection failed
        error: Option<&'a crate::result::Error>,
    },
    /// An event that is emitted while
    /// starting a new transaction
    #[non_exhaustive]
    BeginTransaction {
        /// Transaction level of the newly started
        /// transaction
        depth: NonZeroU32,
    },
    /// An event that is emitted while
    /// committing a transaction
    #[non_exhaustive]
    CommitTransaction {
        /// Transaction level of the to be committed
        /// transaction
        depth: NonZeroU32,
    },
    /// An event that is emitted while
    /// rolling back a transaction
    #[non_exhaustive]
    RollbackTransaction {
        /// Transaction level of the to be rolled
        /// back transaction
        depth: NonZeroU32,
    },
}

// these constructors exist to
// keep `#[non_exhaustive]` on all the variants
// and to gate the constructors on the unstable feature
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
impl<'a> InstrumentationEvent<'a> {
    /// Create a new `InstrumentationEvent::StartEstablishConnection` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn start_establish_connection(url: &'a str) -> Self {
        Self::StartEstablishConnection { url }
    }

    /// Create a new `InstrumentationEvent::FinishEstablishConnection` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn finish_establish_connection(
        url: &'a str,
        error: Option<&'a crate::result::ConnectionError>,
    ) -> Self {
        Self::FinishEstablishConnection { url, error }
    }

    /// Create a new `InstrumentationEvent::StartQuery` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn start_query(query: &'a dyn DebugQuery) -> Self {
        Self::StartQuery { query }
    }

    /// Create a new `InstrumentationEvent::CacheQuery` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn cache_query(sql: &'a str) -> Self {
        Self::CacheQuery { sql }
    }

    /// Create a new `InstrumentationEvent::FinishQuery` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn finish_query(
        query: &'a dyn DebugQuery,
        error: Option<&'a crate::result::Error>,
    ) -> Self {
        Self::FinishQuery { query, error }
    }

    /// Create a new `InstrumentationEvent::BeginTransaction` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn begin_transaction(depth: NonZeroU32) -> Self {
        Self::BeginTransaction { depth }
    }

    /// Create a new `InstrumentationEvent::RollbackTransaction` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn rollback_transaction(depth: NonZeroU32) -> Self {
        Self::RollbackTransaction { depth }
    }

    /// Create a new `InstrumentationEvent::CommitTransaction` event
    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    pub fn commit_transaction(depth: NonZeroU32) -> Self {
        Self::CommitTransaction { depth }
    }
}

/// A type that provides an connection `Instrumentation`
///
/// This trait is the basic building block for logging or
/// otherwise instrumenting diesel connection types. It
/// acts as callback that receives information about certain
/// important connection states
///
/// For simple usages this trait is implemented for closures
/// accepting a [`InstrumentationEvent`] as argument.
///
/// More complex usages and integrations with frameworks like
/// `tracing` and `log` are supposed to be part of their own
/// crates.
pub trait Instrumentation: Downcast + Send + 'static {
    /// The function that is invoked for each event
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>);
}
downcast_rs::impl_downcast!(Instrumentation);

/// Get an instance of the default [`Instrumentation`]
///
/// This function is mostly useful for crates implementing
/// their own connection types
pub fn get_default_instrumentation() -> Option<Box<dyn Instrumentation>> {
    match GLOBAL_INSTRUMENTATION.read() {
        Ok(f) => (*f)(),
        Err(_) => None,
    }
}

/// Set a custom constructor for the default [`Instrumentation`]
/// used by new connections
///
/// ```rust
/// use diesel::connection::{set_default_instrumentation, Instrumentation, InstrumentationEvent};
///
/// // a simple logger that prints all events to stdout
/// fn simple_logger() -> Option<Box<dyn Instrumentation>> {
///     // we need the explicit argument type there due
///     // to bugs in rustc
///     Some(Box::new(|event: InstrumentationEvent<'_>| {
///         println!("{event:?}")
///     }))
/// }
///
/// set_default_instrumentation(simple_logger);
/// ```
pub fn set_default_instrumentation(
    default: fn() -> Option<Box<dyn Instrumentation>>,
) -> crate::QueryResult<()> {
    match GLOBAL_INSTRUMENTATION.write() {
        Ok(mut l) => {
            *l = default;
            Ok(())
        }
        Err(e) => Err(crate::result::Error::DatabaseError(
            crate::result::DatabaseErrorKind::Unknown,
            Box::new(e.to_string()),
        )),
    }
}

impl<F> Instrumentation for F
where
    F: FnMut(InstrumentationEvent<'_>) + Send + 'static,
{
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        (self)(event)
    }
}

impl Instrumentation for Box<dyn Instrumentation> {
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        self.deref_mut().on_connection_event(event)
    }
}

impl<T> Instrumentation for Option<T>
where
    T: Instrumentation,
{
    fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        if let Some(i) = self {
            i.on_connection_event(event)
        }
    }
}

#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
/// An optional dyn instrumentation.
///
/// For ease of use, this type implements [`Deref`] and [`DerefMut`] to `&dyn Instrumentation`,
/// falling back to a no-op implementation if no instrumentation is set.
///
/// The DynInstrumentation type is useful because without it we actually did tend to return
/// (accidentally) &mut Option<Box> as &mut dyn Instrumentation from connection.instrumentation(),
/// so downcasting would have to be done in these two steps by the user, which is counter-intuitive.
pub(crate) struct DynInstrumentation {
    /// zst
    no_instrumentation: NoInstrumentation,
    inner: Option<Box<dyn Instrumentation>>,
}

impl Deref for DynInstrumentation {
    type Target = dyn Instrumentation;

    fn deref(&self) -> &Self::Target {
        self.inner.as_deref().unwrap_or(&self.no_instrumentation)
    }
}

impl DerefMut for DynInstrumentation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .as_deref_mut()
            .unwrap_or(&mut self.no_instrumentation)
    }
}

impl DynInstrumentation {
    /// Create a instance of the default instrumentation provider
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    #[cfg(any(
        feature = "postgres",
        feature = "sqlite",
        feature = "mysql",
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    ))]
    pub(crate) fn default_instrumentation() -> Self {
        Self {
            inner: get_default_instrumentation(),
            no_instrumentation: NoInstrumentation,
        }
    }

    /// Create a noop instrumentation provider instance
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    #[cfg(any(
        feature = "postgres",
        feature = "sqlite",
        feature = "mysql",
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    ))]
    pub(crate) fn none() -> Self {
        Self {
            inner: None,
            no_instrumentation: NoInstrumentation,
        }
    }

    /// register an event with the given instrumentation implementation
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    #[cfg(any(
        feature = "postgres",
        feature = "sqlite",
        feature = "mysql",
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    ))]
    pub(crate) fn on_connection_event(&mut self, event: InstrumentationEvent<'_>) {
        // This implementation is not necessary to be able to call this method on this object
        // because of the already existing Deref impl.
        // However it allows avoiding the dynamic dispatch to the stub value
        if let Some(inner) = self.inner.as_deref_mut() {
            inner.on_connection_event(event)
        }
    }
}

impl<I: Instrumentation> From<I> for DynInstrumentation {
    fn from(instrumentation: I) -> Self {
        Self {
            inner: Some(unpack_instrumentation(Box::new(instrumentation))),
            no_instrumentation: NoInstrumentation,
        }
    }
}

struct NoInstrumentation;

impl Instrumentation for NoInstrumentation {
    fn on_connection_event(&mut self, _: InstrumentationEvent<'_>) {}
}

/// Unwrap unnecessary boxing levels
fn unpack_instrumentation(
    mut instrumentation: Box<dyn Instrumentation>,
) -> Box<dyn Instrumentation> {
    loop {
        match instrumentation.downcast::<Box<dyn Instrumentation>>() {
            Ok(extra_boxed_instrumentation) => instrumentation = *extra_boxed_instrumentation,
            Err(not_extra_boxed_instrumentation) => {
                break not_extra_boxed_instrumentation;
            }
        }
    }
}
