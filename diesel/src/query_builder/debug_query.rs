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

    pub(crate) fn query(&self) -> Result<String, fmt::Error>
    where
        DB: Backend + Default,
        DB::QueryBuilder: Default,
        T: QueryFragment<DB>,
    {
        let mut query_builder = DB::QueryBuilder::default();
        let backend = DB::default();

        // This is not using the `?` operator to reduce the code size of this
        // function, which is getting copies a lot due to monomorphization.
        if QueryFragment::<DB>::to_sql(self.query, &mut query_builder, &backend).is_err() {
            return Err(fmt::Error);
        }

        Ok(query_builder.finish())
    }

    pub(crate) fn binds(&self) -> DebugBinds<'_, T, DB>
    where
        DB: Backend + Default,
        DB::QueryBuilder: Default,
        T: QueryFragment<DB>,
    {
        DebugBinds::<_, DB>::new(self.query)
    }
}

impl<T, DB> Display for DebugQuery<'_, T, DB>
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This is not using the `?` operator to reduce the code size of this
        // function, which is getting copies a lot due to monomorphization.
        let query = match self.query() {
            Ok(query) => query,
            Err(err) => return Err(err),
        };

        let debug_binds = self.binds();
        write!(f, "{} -- binds: {:?}", query, debug_binds)
    }
}

impl<T, DB> Debug for DebugQuery<'_, T, DB>
where
    DB: Backend + Default,
    DB::QueryBuilder: Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This is not using the `?` operator to reduce the code size of this
        // function, which is getting copies a lot due to monomorphization.
        let query = match self.query() {
            Ok(query) => query,
            Err(err) => return Err(err),
        };

        let debug_binds = self.binds();
        f.debug_struct("Query")
            .field("sql", &query)
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

impl<T, DB> Debug for DebugBinds<'_, T, DB>
where
    DB: Backend + Default,
    T: QueryFragment<DB>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend = DB::default();
        let mut buffer = Vec::new();
        let ast_pass = AstPass::debug_binds(&mut buffer, &backend);

        // This is not using the `?` operator to reduce the code size of this
        // function, which is getting copies a lot due to monomorphization.
        if self.query.walk_ast(ast_pass).is_err() {
            return Err(fmt::Error);
        }

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
