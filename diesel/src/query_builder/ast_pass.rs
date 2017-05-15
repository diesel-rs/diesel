use backend::Backend;
use query_builder::{BindCollector, QueryBuilder};
use result::QueryResult;
use types::{ToSql, HasSqlType};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct AstPass<'a, DB> where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::BindCollector: 'a,
{
    internals: AstPassInternals<'a, DB>,
}

impl<'a, DB> AstPass<'a, DB> where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::BindCollector: 'a,
{
    #[cfg_attr(feature="clippy", allow(wrong_self_convention))]
    pub fn to_sql(query_builder: &'a mut DB::QueryBuilder) -> Self {
        AstPass {
            internals: AstPassInternals::ToSql(query_builder),
        }
    }

    pub fn collect_binds(collector: &'a mut DB::BindCollector) -> Self {
        AstPass {
            internals: AstPassInternals::CollectBinds(collector),
        }
    }

    pub fn is_safe_to_cache_prepared(result: &'a mut bool) -> Self {
        AstPass {
            internals: AstPassInternals::IsSafeToCachePrepared(result),
        }
    }

    /// Effectively copies `self`, with a narrower lifetime. This method
    /// matches the semantics of the implicit reborrow that occurs when passing
    /// a reference by value in Rust.
    pub fn reborrow(&mut self) -> AstPass<DB> {
        use self::AstPassInternals::*;
        let internals = match self.internals {
            ToSql(ref mut builder) => ToSql(&mut **builder),
            CollectBinds(ref mut collector) => CollectBinds(&mut **collector),
            IsSafeToCachePrepared(ref mut result) => IsSafeToCachePrepared(&mut **result),
        };
        AstPass { internals }
    }

    pub fn unsafe_to_cache_prepared(&mut self) {
        if let AstPassInternals::IsSafeToCachePrepared(ref mut result) = self.internals {
            **result = false
        }
    }

    pub fn push_sql(&mut self, sql: &str) {
        if let AstPassInternals::ToSql(ref mut builder) = self.internals {
            builder.push_sql(sql);
        }
    }

    pub fn push_identifier(&mut self, identifier: &str) -> QueryResult<()> {
        if let AstPassInternals::ToSql(ref mut builder) = self.internals {
            builder.push_identifier(identifier)?;
        }
        Ok(())
    }

    pub fn push_bind_param<T, U>(&mut self, bind: &U) -> QueryResult<()> where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>,
    {
        use self::AstPassInternals::*;
        match self.internals {
            ToSql(ref mut out) => out.push_bind_param(),
            CollectBinds(ref mut out) => out.push_bound_value(bind)?,
            _ => {}, // noop
        }
        Ok(())
    }

    /// FIXME: This method is a temporary shim, and should be removed when
    /// we are able to merge `InsertValues` into `QueryFragment`
    pub fn query_builder(self) -> Option<&'a mut DB::QueryBuilder> {
        if let AstPassInternals::ToSql(out) = self.internals {
            Some(out)
        } else {
            None
        }
    }

    pub fn push_bind_param_value_only<T, U>(&mut self, bind: &U) -> QueryResult<()> where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>,
    {
        if let AstPassInternals::CollectBinds(..) = self.internals {
            self.push_bind_param(bind)?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
/// This is separate from the struct to cause the enum to be opaque, forcing
/// usage of the methods provided rather than matching on the enum directly.
/// This essentially mimics the capabilities that would be available if
/// `AstPass` were a trait.
enum AstPassInternals<'a, DB> where
    DB: Backend,
    DB::QueryBuilder: 'a,
    DB::BindCollector: 'a,
{
    ToSql(&'a mut DB::QueryBuilder),
    CollectBinds(&'a mut DB::BindCollector),
    IsSafeToCachePrepared(&'a mut bool),
}

