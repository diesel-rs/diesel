use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;
use std::mem;

use super::{AstPass, QueryBuilder, QueryFragment};
use backend::Backend;

/// A struct that implements `fmt::Display` and `fmt::Debug` to show the SQL
/// representation of a query.
///
/// The `Display` implementation will be the exact query sent to the server,
/// plus a comment with the values of the bind parameters. The `Debug`
/// implementation is more structured, and able to be pretty printed.
///
/// See [`debug_query`] for usage examples.
///
/// [`debug_query`]: ../fn.debug_query.html
pub struct DebugQuery<'a, T: 'a, DB> {
    query: &'a T,
    _marker: PhantomData<DB>,
}

impl<'a, T, DB> DebugQuery<'a, T, DB> {
    pub(crate) fn new(query: &'a T) -> Self {
        DebugQuery {
            query,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, DB> Display for DebugQuery<'a, T, DB>
where
    DB: Backend,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut query_builder = DB::QueryBuilder::default();
        QueryFragment::<DB>::to_sql(self.query, &mut query_builder).map_err(|_| fmt::Error)?;
        let debug_binds = DebugBinds::<_, DB>::new(self.query);
        write!(f, "{} -- binds: {:?}", query_builder.finish(), debug_binds)
    }
}

impl<'a, T, DB> Debug for DebugQuery<'a, T, DB>
where
    DB: Backend,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut query_builder = DB::QueryBuilder::default();
        QueryFragment::<DB>::to_sql(self.query, &mut query_builder).map_err(|_| fmt::Error)?;
        let debug_binds = DebugBinds::<_, DB>::new(self.query);
        f.debug_struct("Query")
            .field("sql", &query_builder.finish())
            .field("binds", &debug_binds)
            .finish()
    }
}

/// A struct that implements `fmt::Debug` by walking the given AST and writing
/// the `fmt::Debug` implementation of each bind parameter.
pub struct DebugBinds<'a, T: 'a, DB> {
    query: &'a T,
    _marker: PhantomData<DB>,
}

impl<'a, T, DB> DebugBinds<'a, T, DB> {
    fn new(query: &'a T) -> Self {
        DebugBinds {
            query,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, DB> Debug for DebugBinds<'a, T, DB>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    // Clippy is wrong, this cannot be expressed with pointer casting
    #[allow(clippy::transmute_ptr_to_ptr)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        {
            // Safe because the lifetime is shortened to one smaller
            // than the lifetime of the formatter.
            let list_with_shorter_lifetime = unsafe { mem::transmute(&mut list) };
            let ast_pass = AstPass::debug_binds(list_with_shorter_lifetime);
            self.query.walk_ast(ast_pass).map_err(|_| fmt::Error)?;
        }
        list.finish()?;
        Ok(())
    }
}
