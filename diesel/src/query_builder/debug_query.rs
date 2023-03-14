use super::{AstPass, QueryBuilder, QueryFragment};
use crate::backend::Backend;
use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;

/// A struct that implements `fmt::Display` and `fmt::Debug` to show the SQL
/// representation of a query.
///
/// The `Display` implementation will be the exact query sent to the server,
/// plus a comment with the values of the bind parameters. The `Debug`
/// implementation is more structured, and able to be pretty printed.
///
/// See [`debug_query`] for usage examples.
///
/// [`debug_query`]: crate::query_builder::debug_query()
pub struct DebugQuery<'a, T: 'a, DB> {
    pub(crate) query: &'a T,
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
    DB: Backend + Default,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut query_builder = DB::QueryBuilder::default();
        let backend = DB::default();
        QueryFragment::<DB>::to_sql(self.query, &mut query_builder, &backend)
            .map_err(|_| fmt::Error)?;
        let debug_binds = DebugBinds::<_, DB>::new(self.query);
        write!(f, "{} -- binds: {:?}", query_builder.finish(), debug_binds)
    }
}

impl<'a, T, DB> Debug for DebugQuery<'a, T, DB>
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut query_builder = DB::QueryBuilder::default();
        let backend = DB::default();
        QueryFragment::<DB>::to_sql(self.query, &mut query_builder, &backend)
            .map_err(|_| fmt::Error)?;
        let debug_binds = DebugBinds::<_, DB>::new(self.query);
        f.debug_struct("Query")
            .field("sql", &query_builder.finish())
            .field("binds", &debug_binds)
            .finish()
    }
}

/// A struct that implements `fmt::Debug` by walking the given AST and writing
/// the `fmt::Debug` implementation of each bind parameter.
pub(crate) struct DebugBinds<'a, T: 'a, DB> {
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
    DB: Backend + Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend = DB::default();
        let mut buffer = Vec::new();
        let ast_pass = AstPass::debug_binds(&mut buffer, &backend);
        self.query.walk_ast(ast_pass).map_err(|_| fmt::Error)?;

        let mut list = f.debug_list();
        for entry in buffer {
            list.entry(&entry);
        }
        list.finish()?;
        Ok(())
    }
}
