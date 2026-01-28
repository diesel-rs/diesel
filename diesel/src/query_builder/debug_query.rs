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

fn serialize_query<DB>(query: &dyn QueryFragment<DB>) -> Result<String, fmt::Error>
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
{
    let mut query_builder = DB::QueryBuilder::default();
    let backend = DB::default();
    QueryFragment::<DB>::to_sql(query, &mut query_builder, &backend).map_err(|_| fmt::Error)?;
    Ok(query_builder.finish())
}

fn display<DB>(query: &dyn QueryFragment<DB>, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
{
    let debug_binds = DebugBinds::<DB>::new(query);
    let query = serialize_query(query)?;
    write!(f, "{query} -- binds: {debug_binds:?}")
}

fn debug<DB>(query: &dyn QueryFragment<DB>, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
{
    let debug_binds = DebugBinds::<DB>::new(query);
    let query = serialize_query(query)?;
    f.debug_struct("Query")
        .field("sql", &query)
        .field("binds", &debug_binds)
        .finish()
}

impl<T, DB> Display for DebugQuery<'_, T, DB>
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display(self.query, f)
    }
}

impl<T, DB> Debug for DebugQuery<'_, T, DB>
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug(self.query, f)
    }
}

/// A struct that implements `fmt::Debug` by walking the given AST and writing
/// the `fmt::Debug` implementation of each bind parameter.
pub(crate) struct DebugBinds<'a, DB> {
    query: &'a dyn QueryFragment<DB>,
}

impl<'a, DB> DebugBinds<'a, DB>
where
    DB: Backend,
{
    fn new(query: &'a dyn QueryFragment<DB>) -> Self {
        DebugBinds { query }
    }
}

impl<DB> Debug for DebugBinds<'_, DB>
where
    DB: Backend + Default,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend = DB::default();
        let mut buffer = Vec::new();
        let ast_pass = AstPass::debug_binds(&mut buffer, &backend);
        self.query.walk_ast(ast_pass).map_err(|_| fmt::Error)?;
        format_list(f, &buffer)
    }
}

fn format_list<'b>(f: &mut fmt::Formatter<'_>, entries: &[Box<dyn Debug + 'b>]) -> fmt::Result {
    let mut list = f.debug_list();
    for entry in entries {
        list.entry(entry);
    }
    list.finish()
}
