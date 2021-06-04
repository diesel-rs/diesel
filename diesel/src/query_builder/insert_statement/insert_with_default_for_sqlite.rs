use super::batch_insert::AsValueIterator;
use super::{BatchInsert, InsertStatement};
use crate::connection::Connection;
use crate::insertable::Insertable;
use crate::query_builder::returning_clause::NoReturningClause;
use crate::query_builder::{DebugQuery, QueryFragment, QueryId};
use crate::query_dsl::methods::ExecuteDsl;
use crate::sqlite::Sqlite;
use crate::{QueryResult, SqliteConnection, Table};
use std::fmt::{self, Debug, Display, Write};

pub trait SqliteInsertableQueryfragment<Tab, Op, C>
where
    Self: Insertable<Tab>,
{
    fn execute_single_record(
        v: Self::Values,
        conn: &mut C,
        target: Tab,
        op: Op,
    ) -> QueryResult<usize>;

    fn write_debug_query(v: Self::Values, out: &mut impl Write, target: Tab, op: Op)
        -> fmt::Result;
}

impl<'a, Tab, T, Op, C> SqliteInsertableQueryfragment<Tab, Op, C> for &'a T
where
    Self: Insertable<Tab>,
    Tab: Table,
    C: Connection<Backend = Sqlite>,
    InsertStatement<Tab, <&'a T as Insertable<Tab>>::Values, Op>:
        ExecuteDsl<C, Sqlite> + QueryFragment<Sqlite>,
{
    fn execute_single_record(
        v: Self::Values,
        conn: &mut C,
        target: Tab,
        op: Op,
    ) -> QueryResult<usize> {
        ExecuteDsl::execute(InsertStatement::new(target, v, op, NoReturningClause), conn)
    }

    fn write_debug_query(
        v: Self::Values,
        out: &mut impl Write,
        target: Tab,
        op: Op,
    ) -> fmt::Result {
        let stmt = InsertStatement::new(target, v, op, NoReturningClause);
        write!(out, "{}", crate::debug_query::<Sqlite, _>(&stmt))
    }
}

impl<T, V, QId, Op, C, const STATIC_QUERY_ID: bool> ExecuteDsl<C, Sqlite>
    for InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>
where
    C: Connection<Backend = Sqlite>,
    V: AsValueIterator<T>,
    for<'a> &'a V::Item: SqliteInsertableQueryfragment<T, Op, C>,
    T: Table + Copy + QueryId,
    T::FromClause: QueryFragment<Sqlite>,
    Op: Copy + QueryId,
{
    fn execute(query: Self, conn: &mut C) -> QueryResult<usize> {
        conn.transaction(|conn| {
            let mut result = 0;
            for record in query.records.values.as_value_iter() {
                result +=
                    <&V::Item as SqliteInsertableQueryfragment<T, Op, C>>::execute_single_record(
                        record,
                        conn,
                        query.target,
                        query.operator,
                    )?;
            }
            Ok(result)
        })
    }
}

impl<'a, T, V, QId, Op, const STATIC_QUERY_ID: bool> Display
    for DebugQuery<'a, InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>, Sqlite>
where
    V: AsValueIterator<T>,
    for<'b> &'b V::Item: SqliteInsertableQueryfragment<T, Op, SqliteConnection>,
    T: Copy,
    Op: Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "BEGIN;")?;
        for record in self.query.records.values.as_value_iter() {
            <&V::Item as SqliteInsertableQueryfragment<T, Op, SqliteConnection>>::write_debug_query(record, f, self.query.target, self.query.operator)?;
            writeln!(f)?;
        }
        writeln!(f, "COMMIT;")?;
        Ok(())
    }
}

impl<'a, T, V, QId, Op, const STATIC_QUERY_ID: bool> Debug
    for DebugQuery<'a, InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>, Sqlite>
where
    V: AsValueIterator<T>,
    for<'b> &'b V::Item: SqliteInsertableQueryfragment<T, Op, SqliteConnection>,
    T: Copy,
    Op: Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut statements = vec![String::from("BEGIN")];
        for record in self.query.records.values.as_value_iter() {
            let mut out = String::new();
            <&V::Item as SqliteInsertableQueryfragment<T, Op, SqliteConnection>>::write_debug_query(record, &mut out, self.query.target, self.query.operator)?;
            statements.push(out);
        }
        statements.push("COMMIT".into());

        f.debug_struct("Query")
            .field("sql", &statements)
            .field("binds", &[] as &[i32; 0])
            .finish()
    }
}
