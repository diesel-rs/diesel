use std::fmt;

use crate::backend::Backend;
use crate::query_builder::{BindCollector, MoveableBindCollector, QueryBuilder};
use crate::result::QueryResult;
use crate::serialize::ToSql;
use crate::sql_types::HasSqlType;

#[allow(missing_debug_implementations)]
/// The primary type used when walking a Diesel AST during query execution.
///
/// Executing a query is generally done in multiple passes. This list includes,
/// but is not limited to:
///
/// - Generating the SQL
/// - Collecting and serializing bound values (sent separately from the SQL)
/// - Determining if a query is safe to store in the prepared statement cache
///
/// When adding a new type that is used in a Diesel AST, you don't need to care
/// about which specific passes are being performed, nor is there any way for
/// you to find out what the current pass is. You should simply call the
/// relevant methods and trust that they will be a no-op if they're not relevant
/// to the current pass.
pub struct AstPass<'a, 'b, DB>
where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::MetadataLookup: 'a,
    'b: 'a,
{
    internals: AstPassInternals<'a, 'b, DB>,
    backend: &'b DB,
}

impl<'a, 'b, DB> AstPass<'a, 'b, DB>
where
    DB: Backend,
    'b: 'a,
{
    pub(crate) fn to_sql(
        query_builder: &'a mut DB::QueryBuilder,
        options: &'a mut AstPassToSqlOptions,
        backend: &'b DB,
    ) -> Self {
        AstPass {
            internals: AstPassInternals::ToSql(query_builder, options),
            backend,
        }
    }

    pub(crate) fn collect_binds(
        collector: &'a mut DB::BindCollector<'b>,
        metadata_lookup: &'a mut DB::MetadataLookup,
        backend: &'b DB,
    ) -> Self {
        AstPass {
            internals: AstPassInternals::CollectBinds {
                collector,
                metadata_lookup,
            },
            backend,
        }
    }

    pub(crate) fn is_safe_to_cache_prepared(result: &'a mut bool, backend: &'b DB) -> Self {
        AstPass {
            internals: AstPassInternals::IsSafeToCachePrepared(result),
            backend,
        }
    }

    pub(crate) fn debug_binds(
        formatter: &'a mut Vec<Box<dyn fmt::Debug + 'b>>,
        backend: &'b DB,
    ) -> Self {
        AstPass {
            internals: AstPassInternals::DebugBinds(formatter),
            backend,
        }
    }

    /// Does running this AST pass have any effect?
    ///
    /// The result will be set to `false` if any method that generates SQL
    /// is called.
    pub(crate) fn is_noop(result: &'a mut bool, backend: &'b DB) -> Self {
        AstPass {
            internals: AstPassInternals::IsNoop(result),
            backend,
        }
    }

    #[cfg(feature = "sqlite")]
    pub(crate) fn skip_from(&mut self, value: bool) {
        if let AstPassInternals::ToSql(_, ref mut options) = self.internals {
            options.skip_from = value
        }
    }

    /// Call this method whenever you pass an instance of `AstPass` by value.
    ///
    /// Effectively copies `self`, with a narrower lifetime. When passing a
    /// reference or a mutable reference, this is normally done by rust
    /// implicitly. This is why you can pass `&mut Foo` to multiple functions,
    /// even though mutable references are not `Copy`. However, this is only
    /// done implicitly for references. For structs with lifetimes it must be
    /// done explicitly. This method matches the semantics of what Rust would do
    /// implicitly if you were passing a mutable reference
    #[allow(clippy::explicit_auto_deref)] // clippy is wrong here
    pub fn reborrow(&'_ mut self) -> AstPass<'_, 'b, DB> {
        let internals = match self.internals {
            AstPassInternals::ToSql(ref mut builder, ref mut options) => {
                AstPassInternals::ToSql(*builder, options)
            }
            AstPassInternals::CollectBinds {
                ref mut collector,
                ref mut metadata_lookup,
            } => AstPassInternals::CollectBinds {
                collector: *collector,
                metadata_lookup: *metadata_lookup,
            },
            AstPassInternals::IsSafeToCachePrepared(ref mut result) => {
                AstPassInternals::IsSafeToCachePrepared(result)
            }
            AstPassInternals::DebugBinds(ref mut f) => AstPassInternals::DebugBinds(f),
            AstPassInternals::IsNoop(ref mut result) => AstPassInternals::IsNoop(result),
        };
        AstPass {
            internals,
            backend: self.backend,
        }
    }

    /// Mark the current query being constructed as unsafe to store in the
    /// prepared statement cache.
    ///
    /// Diesel caches prepared statements as much as possible. However, it is
    /// important to ensure that this doesn't result in unbounded memory usage
    /// on the database server. To ensure this is the case, any logical query
    /// which could generate a potentially unbounded number of prepared
    /// statements *must* call this method. Examples of AST nodes which do this
    /// are:
    ///
    /// - `SqlLiteral`. We have no way of knowing if the SQL string was
    ///   constructed dynamically or not, so we must assume it was dynamic.
    /// - `EqAny` when passed a Rust `Vec`. The `IN` operator requires one bind
    ///   parameter per element, meaning that the query could generate up to
    ///   `usize` unique prepared statements.
    /// - `InsertStatement`. Unbounded due to the variable number of records
    ///   being inserted generating unique SQL.
    /// - `UpdateStatement`. The number of potential queries is actually
    ///   technically bounded, but the upper bound is the number of columns on
    ///   the table factorial which is too large to be safe.
    pub fn unsafe_to_cache_prepared(&mut self) {
        if let AstPassInternals::IsSafeToCachePrepared(ref mut result) = self.internals {
            **result = false
        }
    }

    /// Push the given SQL string on the end of the query being constructed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::query_builder::{QueryFragment, AstPass};
    /// # use diesel::backend::Backend;
    /// # use diesel::QueryResult;
    /// # struct And<Left, Right> { left: Left, right: Right }
    /// impl<Left, Right, DB> QueryFragment<DB> for And<Left, Right>
    /// where
    ///     DB: Backend,
    ///     Left: QueryFragment<DB>,
    ///     Right: QueryFragment<DB>,
    /// {
    ///     fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
    ///         self.left.walk_ast(out.reborrow())?;
    ///         out.push_sql(" AND ");
    ///         self.right.walk_ast(out.reborrow())?;
    ///         Ok(())
    ///     }
    /// }
    /// # fn main() {}
    /// ```
    pub fn push_sql(&mut self, sql: &str) {
        match self.internals {
            AstPassInternals::ToSql(ref mut builder, _) => builder.push_sql(sql),
            AstPassInternals::IsNoop(ref mut result) => **result = false,
            _ => {}
        }
    }

    /// Push the given SQL identifier on the end of the query being constructed.
    ///
    /// The identifier will be quoted using the rules specific to the backend
    /// the query is being constructed for.
    pub fn push_identifier(&mut self, identifier: &str) -> QueryResult<()> {
        match self.internals {
            AstPassInternals::ToSql(ref mut builder, _) => builder.push_identifier(identifier)?,
            AstPassInternals::IsNoop(ref mut result) => **result = false,
            _ => {}
        }
        Ok(())
    }

    /// Push a value onto the given query to be sent separate from the SQL
    ///
    /// This method affects multiple AST passes. It should be called at the
    /// point in the query where you'd want the parameter placeholder (`$1` on
    /// PG, `?` on other backends) to be inserted.
    pub fn push_bind_param<T, U>(&mut self, bind: &'b U) -> QueryResult<()>
    where
        DB: HasSqlType<T>,
        U: ToSql<T, DB> + ?Sized,
    {
        match self.internals {
            AstPassInternals::ToSql(ref mut out, _) => out.push_bind_param(),
            AstPassInternals::CollectBinds {
                ref mut collector,
                ref mut metadata_lookup,
            } => collector.push_bound_value(bind, metadata_lookup)?,
            AstPassInternals::DebugBinds(ref mut f) => {
                f.push(Box::new(bind));
            }
            AstPassInternals::IsNoop(ref mut result) => **result = false,
            _ => {}
        }
        Ok(())
    }

    /// Push a value onto the given query to be sent separate from the SQL
    ///
    /// This method affects multiple AST passes. It should be called at the
    /// point in the raw SQL is inserted. This assumes the parameter placeholder
    /// (`$1` on PG, `?` on other backends) is already inserted.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub(crate) fn push_bind_param_value_only<T, U>(&mut self, bind: &'b U) -> QueryResult<()>
    where
        DB: HasSqlType<T>,
        U: ToSql<T, DB> + ?Sized,
    {
        match self.internals {
            AstPassInternals::CollectBinds { .. } | AstPassInternals::DebugBinds(..) => {
                self.push_bind_param(bind)?
            }
            AstPassInternals::ToSql(ref mut out, _) => {
                out.push_bind_param_value_only();
            }
            _ => {}
        }
        Ok(())
    }

    /// Push bind collector values from its data onto the query
    ///
    /// This method works with [MoveableBindCollector] data [MoveableBindCollector::BindData]
    /// and is used with already collected query meaning its SQL is already built and its
    /// bind data already collected.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub(crate) fn push_bind_collector_data<MD>(
        &mut self,
        bind_collector_data: &MD,
    ) -> QueryResult<()>
    where
        DB: Backend,
        for<'bc> DB::BindCollector<'bc>: MoveableBindCollector<DB, BindData = MD>,
    {
        match self.internals {
            AstPassInternals::CollectBinds {
                ref mut collector,
                metadata_lookup: _,
            } => collector.append_bind_data(bind_collector_data),
            AstPassInternals::DebugBinds(ref mut f) => {
                f.push(Box::new("Opaque bind collector data"))
            }
            _ => {}
        }
        Ok(())
    }

    /// Get information about the backend that will consume this query
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc(hidden)
    )] // This is used by the `define_sql_function` macro
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub fn backend(&self) -> &DB {
        self.backend
    }

    /// Get if the query should be rendered with from clauses or not
    #[cfg_attr(
        not(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"),
        doc(hidden)
    )] // This is used by the `__diesel_column` macro
    #[cfg_attr(
        docsrs,
        doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
    )]
    pub fn should_skip_from(&self) -> bool {
        if let AstPassInternals::ToSql(_, ref options) = self.internals {
            options.skip_from
        } else {
            false
        }
    }
}

#[allow(missing_debug_implementations)]
/// This is separate from the struct to cause the enum to be opaque, forcing
/// usage of the methods provided rather than matching on the enum directly.
/// This essentially mimics the capabilities that would be available if
/// `AstPass` were a trait.
enum AstPassInternals<'a, 'b, DB>
where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::MetadataLookup: 'a,
    'b: 'a,
{
    ToSql(&'a mut DB::QueryBuilder, &'a mut AstPassToSqlOptions),
    CollectBinds {
        collector: &'a mut DB::BindCollector<'b>,
        metadata_lookup: &'a mut DB::MetadataLookup,
    },
    IsSafeToCachePrepared(&'a mut bool),
    DebugBinds(&'a mut Vec<Box<dyn fmt::Debug + 'b>>),
    IsNoop(&'a mut bool),
}

#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
#[allow(missing_debug_implementations)]
#[allow(missing_copy_implementations)]
#[derive(Default)]
/// This is used to pass down additional settings to the `AstPass`
/// when rendering the sql string.
pub(crate) struct AstPassToSqlOptions {
    skip_from: bool,
}

/// This is an internal extension trait with methods required for
/// `#[derive(MultiConnection)]`
pub trait AstPassHelper<'a, 'b, DB>
where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::MetadataLookup: 'a,
    'b: 'a,
{
    /// This function converts the given `AstPass` instance to
    /// an `AstPass` instance for another database system. This requires that the
    /// given instance contains compatible BindCollector/QueryBuilder/… implementations
    /// for the target backend. We use explicit conversion functions here instead of relaying on
    /// `From` impls because generating them as part of `#[derive(MultiConnection)]` is not possible
    /// due to [compiler bugs](https://github.com/rust-lang/rust/issues/100712)
    fn cast_database<DB2>(
        self,
        convert_bind_collector: impl Fn(&'a mut DB::BindCollector<'b>) -> &'a mut DB2::BindCollector<'b>,
        convert_query_builder: impl Fn(&mut DB::QueryBuilder) -> &mut DB2::QueryBuilder,
        convert_backend: impl Fn(&DB) -> &DB2,
        convert_lookup: impl Fn(&'a mut DB::MetadataLookup) -> &'a mut DB2::MetadataLookup,
    ) -> AstPass<'a, 'b, DB2>
    where
        DB2: Backend,
        DB2::QueryBuilder: 'a,
        DB2::MetadataLookup: 'a,
        'b: 'a;

    /// This function allows to access the inner bind collector if
    /// this `AstPass` represents a collect binds pass.
    fn bind_collector(&mut self) -> Option<(&mut DB::BindCollector<'b>, &mut DB::MetadataLookup)>;
}

impl<'a, 'b, DB> AstPassHelper<'a, 'b, DB> for AstPass<'a, 'b, DB>
where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::MetadataLookup: 'a,
    'b: 'a,
{
    fn cast_database<DB2>(
        self,
        convert_bind_collector: impl Fn(&'a mut DB::BindCollector<'b>) -> &'a mut DB2::BindCollector<'b>,
        convert_query_builder: impl Fn(&mut DB::QueryBuilder) -> &mut DB2::QueryBuilder,
        convert_backend: impl Fn(&DB) -> &DB2,
        convert_lookup: impl Fn(&'a mut DB::MetadataLookup) -> &'a mut DB2::MetadataLookup,
    ) -> AstPass<'a, 'b, DB2>
    where
        DB2: Backend,
        DB2::QueryBuilder: 'a,
        DB2::MetadataLookup: 'a,
        'b: 'a,
    {
        let casted_pass = match self.internals {
            AstPassInternals::ToSql(qb, opts) => {
                AstPassInternals::ToSql(convert_query_builder(qb), opts)
            }
            AstPassInternals::CollectBinds {
                collector,
                metadata_lookup,
            } => AstPassInternals::CollectBinds {
                collector: convert_bind_collector(collector),
                metadata_lookup: convert_lookup(metadata_lookup),
            },
            AstPassInternals::IsSafeToCachePrepared(b) => {
                AstPassInternals::IsSafeToCachePrepared(b)
            }
            AstPassInternals::DebugBinds(b) => AstPassInternals::DebugBinds(b),
            AstPassInternals::IsNoop(b) => AstPassInternals::IsNoop(b),
        };

        AstPass {
            internals: casted_pass,
            backend: convert_backend(self.backend),
        }
    }

    fn bind_collector(&mut self) -> Option<(&mut DB::BindCollector<'b>, &mut DB::MetadataLookup)> {
        if let AstPassInternals::CollectBinds {
            collector,
            metadata_lookup,
        } = &mut self.internals
        {
            Some((collector, metadata_lookup))
        } else {
            None
        }
    }
}
