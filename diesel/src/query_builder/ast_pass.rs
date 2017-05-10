use backend::Backend;
use query_builder::BindCollector;
use result::QueryResult;
use types::{ToSql, HasSqlType};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct AstPass<'a, DB> where
    DB: Backend,
    DB::BindCollector: 'a,
{
    internals: AstPassInternals<'a, DB>,
}

impl<'a, DB> AstPass<'a, DB> where
    DB: Backend,
    DB::BindCollector: 'a,
{
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

    pub fn push_bind_param<T, U>(&mut self, bind: &U) -> QueryResult<()> where
        DB: HasSqlType<T>,
        U: ToSql<T, DB>,
    {
        if let AstPassInternals::CollectBinds(ref mut out) = self.internals {
            out.push_bound_value(bind)?;
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
    DB::BindCollector: 'a,
{
    CollectBinds(&'a mut DB::BindCollector),
    IsSafeToCachePrepared(&'a mut bool),
}

