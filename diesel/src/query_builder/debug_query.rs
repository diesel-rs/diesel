use super::{AstPass, QueryBuilder, QueryFragment};
use crate::backend::Backend;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};
use core::marker::PhantomData;

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

fn fmt_query<DB>(
    query: &dyn QueryFragment<DB>,
    f: &mut fmt::Formatter<'_>,
    formatter: fn(String, &DebugBinds<'_>, f: &mut fmt::Formatter<'_>) -> fmt::Result,
) -> fmt::Result
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
{
    let backend = DB::default();
    let mut buffer = Vec::new();
    let ast_pass = AstPass::debug_binds(&mut buffer, &backend);
    query.walk_ast(ast_pass).map_err(|_| fmt::Error)?;
    let debug_binds = DebugBinds::new(&buffer);
    let query = serialize_query(query)?;
    formatter(query, &debug_binds, f)
}

pub(crate) fn display(
    query: String,
    debug_binds: &DebugBinds<'_>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    write!(f, "{query} -- binds: {debug_binds:?}")
}

pub(crate) fn debug(
    query: String,
    debug_binds: &DebugBinds<'_>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
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
        fmt_query(self.query, f, display)
    }
}

impl<T, DB> Debug for DebugQuery<'_, T, DB>
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_query(self.query, f, debug)
    }
}

/// A struct that implements `fmt::Debug` by walking the given AST and writing
/// the `fmt::Debug` implementation of each bind parameter.
pub(crate) struct DebugBinds<'a> {
    binds: &'a [Box<dyn Debug + 'a>],
}

impl<'a> DebugBinds<'a> {
    pub(crate) fn new(binds: &'a [Box<dyn Debug + 'a>]) -> Self {
        DebugBinds { binds }
    }
}

impl Debug for DebugBinds<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_list(f, self.binds)
    }
}

fn format_list<'b>(f: &mut fmt::Formatter<'_>, entries: &[Box<dyn Debug + 'b>]) -> fmt::Result {
    let mut list = f.debug_list();
    for entry in entries {
        list.entry(entry);
    }
    list.finish()
}
