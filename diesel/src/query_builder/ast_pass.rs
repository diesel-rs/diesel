use std::{fmt, mem};

use backend::Backend;
use query_builder::{BindCollector, QueryBuilder};
use result::QueryResult;
use types::{HasSqlType, ToSql};

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
pub struct AstPass<'a, DB>
where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::BindCollector: 'a,
    DB::MetadataLookup: 'a,
{
    internals: AstPassInternals<'a, DB>,
}

impl<'a, DB> AstPass<'a, DB>
where
    DB: Backend,
{
    #[doc(hidden)]
    #[cfg_attr(feature = "clippy", allow(wrong_self_convention))]
    pub fn to_sql(query_builder: &'a mut DB::QueryBuilder) -> Self {
        AstPass {
            internals: AstPassInternals::ToSql(query_builder),
        }
    }

    #[doc(hidden)]
    pub fn collect_binds(
        collector: &'a mut DB::BindCollector,
        metadata_lookup: &'a DB::MetadataLookup,
    ) -> Self {
        AstPass {
            internals: AstPassInternals::CollectBinds {
                collector,
                metadata_lookup,
            },
        }
    }

    #[doc(hidden)]
    pub fn is_safe_to_cache_prepared(result: &'a mut bool) -> Self {
        AstPass {
            internals: AstPassInternals::IsSafeToCachePrepared(result),
        }
    }

    #[doc(hidden)]
    pub fn debug_binds(formatter: &'a mut fmt::DebugList<'a, 'a>) -> Self {
        AstPass {
            internals: AstPassInternals::DebugBinds(formatter),
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
    pub fn reborrow(&mut self) -> AstPass<DB> {
        use self::AstPassInternals::*;
        let internals = match self.internals {
            ToSql(ref mut builder) => ToSql(&mut **builder),
            CollectBinds {
                ref mut collector,
                metadata_lookup,
            } => CollectBinds {
                collector: &mut **collector,
                metadata_lookup: &*metadata_lookup,
            },
            IsSafeToCachePrepared(ref mut result) => IsSafeToCachePrepared(&mut **result),
            DebugBinds(ref mut f) => {
                // Safe because the lifetime is always being shortened.
                let f_with_shorter_lifetime = unsafe { mem::transmute(&mut **f) };
                DebugBinds(f_with_shorter_lifetime)
            }
        };
        AstPass { internals }
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
    /// # extern crate diesel;
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
    ///     fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
    ///         self.left.walk_ast(out.reborrow())?;
    ///         out.push_sql(" AND ");
    ///         self.right.walk_ast(out.reborrow())?;
    ///         Ok(())
    ///     }
    /// }
    /// # fn main() {}
    /// ```
    pub fn push_sql(&mut self, sql: &str) {
        if let AstPassInternals::ToSql(ref mut builder) = self.internals {
            builder.push_sql(sql);
        }
    }

    /// Push the given SQL identifier on the end of the query being constructed.
    ///
    /// The identifier will be quoted using the rules specific to the backend
    /// the query is being constructed for.
    pub fn push_identifier(&mut self, identifier: &str) -> QueryResult<()> {
        if let AstPassInternals::ToSql(ref mut builder) = self.internals {
            builder.push_identifier(identifier)?;
        }
        Ok(())
    }

    /// Push a value onto the given query to be sent separate from the SQL
    ///
    /// This method affects multiple AST passes. It should be called at the
    /// point in the query where you'd want the parameter placeholder (`$1` on
    /// PG, `?` on other backends) to be inserted.
    pub fn push_bind_param<T, U>(&mut self, bind: &U) -> QueryResult<()>
    where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>,
    {
        use self::AstPassInternals::*;
        match self.internals {
            ToSql(ref mut out) => out.push_bind_param(),
            CollectBinds {
                ref mut collector,
                metadata_lookup,
            } => collector.push_bound_value(bind, metadata_lookup)?,
            DebugBinds(ref mut f) => {
                f.entry(bind);
            }
            _ => {} // noop
        }
        Ok(())
    }

    /// FIXME: This method is a temporary shim, and should be removed when
    /// we are able to merge `InsertValues` into `QueryFragment`
    #[doc(hidden)]
    pub fn query_builder(self) -> Option<&'a mut DB::QueryBuilder> {
        if let AstPassInternals::ToSql(out) = self.internals {
            Some(out)
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub fn push_bind_param_value_only<T, U>(&mut self, bind: &U) -> QueryResult<()>
    where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>,
    {
        use self::AstPassInternals::*;
        match self.internals {
            CollectBinds { .. } | DebugBinds(..) => self.push_bind_param(bind)?,
            _ => {}
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
/// This is separate from the struct to cause the enum to be opaque, forcing
/// usage of the methods provided rather than matching on the enum directly.
/// This essentially mimics the capabilities that would be available if
/// `AstPass` were a trait.
enum AstPassInternals<'a, DB>
where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::BindCollector: 'a,
    DB::MetadataLookup: 'a,
{
    ToSql(&'a mut DB::QueryBuilder),
    CollectBinds {
        collector: &'a mut DB::BindCollector,
        metadata_lookup: &'a DB::MetadataLookup,
    },
    IsSafeToCachePrepared(&'a mut bool),
    DebugBinds(&'a mut fmt::DebugList<'a, 'a>),
}
