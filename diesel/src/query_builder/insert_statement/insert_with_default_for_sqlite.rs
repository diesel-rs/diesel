use super::{BatchInsert, InsertStatement};
use crate::connection::Connection;
use crate::insertable::Insertable;
use crate::insertable::{CanInsertInSingleQuery, ColumnInsertValue, DefaultableColumnInsertValue};
use crate::prelude::*;
use crate::query_builder::returning_clause::NoReturningClause;
use crate::query_builder::{
    AsValueIterator, AstPass, InsertableQueryfragment, QueryId, ValuesClause,
};
use crate::query_builder::{DebugQuery, QueryFragment};
use crate::query_dsl::methods::ExecuteDsl;
use crate::sqlite::Sqlite;
use crate::{QueryResult, Table};
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

pub trait DebugQueryHelper<ContainsDefaultableValue> {
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn fmt_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<'a, T, V, QId, Op, const STATIC_QUERY_ID: bool> DebugQueryHelper<Yes>
    for DebugQuery<'a, InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>, Sqlite>
where
    V: AsValueIterator<T>,
    for<'b> &'b V::Item: SqliteInsertableQueryfragment<T, Op, SqliteConnection>,
    T: Copy,
    Op: Copy,
{
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    fn fmt_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BEGIN;")?;
        for record in self.query.records.values.as_value_iter() {
            <&V::Item as SqliteInsertableQueryfragment<T, Op, SqliteConnection>>::write_debug_query(record, f, self.query.target, self.query.operator)?;
            writeln!(f)?;
        }
        writeln!(f, "COMMIT;")?;
        Ok(())
    }
}

impl<'a, T, V, QId, Op, const STATIC_QUERY_ID: bool> DebugQueryHelper<No>
    for DebugQuery<'a, InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>, Sqlite>
where
    T: Copy,
    Op: Copy,
    DebugQuery<
        'a,
        InsertStatement<T, SqliteBatchInsertWrapper<V, T, QId, STATIC_QUERY_ID>, Op>,
        Sqlite,
    >: Debug + Display,
{
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = unsafe {
            // This cast is safe as `SqliteBatchInsertWrapper` is #[repr(transparent)]
            &*(self as *const DebugQuery<
                InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>,
                Sqlite,
            >
                as *const DebugQuery<
                    InsertStatement<T, SqliteBatchInsertWrapper<V, T, QId, STATIC_QUERY_ID>, Op>,
                    Sqlite,
                >)
        };
        <_ as Debug>::fmt(value, f)
    }

    fn fmt_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = unsafe {
            // This cast is safe as `SqliteBatchInsertWrapper` is #[repr(transparent)]
            &*(self as *const DebugQuery<
                InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>,
                Sqlite,
            >
                as *const DebugQuery<
                    InsertStatement<T, SqliteBatchInsertWrapper<V, T, QId, STATIC_QUERY_ID>, Op>,
                    Sqlite,
                >)
        };
        <_ as Display>::fmt(value, f)
    }
}

impl<'a, T, V, QId, Op, O, const STATIC_QUERY_ID: bool> Display
    for DebugQuery<'a, InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>, Sqlite>
where
    V: AsValueIterator<T>,
    V::Item: Insertable<T>,
    <V::Item as Insertable<T>>::Values: ContainsDefaultableValue<Out = O>,
    Self: DebugQueryHelper<O>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_display(f)
    }
}

impl<'a, T, V, QId, Op, O, const STATIC_QUERY_ID: bool> Debug
    for DebugQuery<'a, InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>, Sqlite>
where
    V: AsValueIterator<T>,
    V::Item: Insertable<T>,
    <V::Item as Insertable<T>>::Values: ContainsDefaultableValue<Out = O>,
    Self: DebugQueryHelper<O>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_debug(f)
    }
}

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Yes;

impl Default for Yes {
    fn default() -> Self {
        Yes
    }
}

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct No;

impl Default for No {
    fn default() -> Self {
        No
    }
}

pub trait Any<Rhs> {
    type Out: Any<Yes> + Any<No>;
}

impl Any<No> for No {
    type Out = No;
}

impl Any<Yes> for No {
    type Out = Yes;
}

impl Any<No> for Yes {
    type Out = Yes;
}

impl Any<Yes> for Yes {
    type Out = Yes;
}

pub trait ContainsDefaultableValue {
    type Out: Any<Yes> + Any<No>;
}

impl<C, B> ContainsDefaultableValue for ColumnInsertValue<C, B> {
    type Out = No;
}

impl<I> ContainsDefaultableValue for DefaultableColumnInsertValue<I> {
    type Out = Yes;
}

impl<I, const SIZE: usize> ContainsDefaultableValue for [I; SIZE]
where
    I: ContainsDefaultableValue,
{
    type Out = I::Out;
}

impl<I, T> ContainsDefaultableValue for ValuesClause<I, T>
where
    I: ContainsDefaultableValue,
{
    type Out = I::Out;
}

impl<'a, T> ContainsDefaultableValue for &'a T
where
    T: ContainsDefaultableValue,
{
    type Out = T::Out;
}

impl<V, T, QId, C, Op, O, const STATIC_QUERY_ID: bool> ExecuteDsl<C, Sqlite>
    for InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>
where
    C: Connection<Backend = Sqlite>,
    V: AsValueIterator<T>,
    V::Item: Insertable<T>,
    <V::Item as Insertable<T>>::Values: ContainsDefaultableValue<Out = O>,
    O: Default,
    (O, Self): ExecuteDsl<C, Sqlite>,
{
    fn execute(query: Self, conn: &mut C) -> QueryResult<usize> {
        <(O, Self) as ExecuteDsl<C, Sqlite>>::execute((O::default(), query), conn)
    }
}

impl<V, T, QId, C, Op, const STATIC_QUERY_ID: bool> ExecuteDsl<C, Sqlite>
    for (
        Yes,
        InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>,
    )
where
    C: Connection<Backend = Sqlite>,
    V: AsValueIterator<T>,
    for<'a> &'a V::Item: SqliteInsertableQueryfragment<T, Op, C>,
    T: Table + Copy + QueryId,
    T::FromClause: QueryFragment<Sqlite>,
    Op: Copy + QueryId,
{
    fn execute((Yes, query): Self, conn: &mut C) -> QueryResult<usize> {
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

#[allow(missing_debug_implementations, missing_copy_implementations)]
#[repr(transparent)]
pub struct SqliteBatchInsertWrapper<V, T, QId, const STATIC_QUERY_ID: bool>(
    BatchInsert<V, T, QId, STATIC_QUERY_ID>,
);

impl<V, Tab, T, QId, const STATIC_QUERY_ID: bool> QueryFragment<Sqlite>
    for SqliteBatchInsertWrapper<V, Tab, QId, STATIC_QUERY_ID>
where
    V: AsValueIterator<Tab, Item = T>,
    for<'a> &'a T: InsertableQueryfragment<Tab, Sqlite>,
{
    fn walk_ast(&self, mut out: AstPass<Sqlite>) -> QueryResult<()> {
        if !STATIC_QUERY_ID {
            out.unsafe_to_cache_prepared();
        }

        let mut values = self.0.values.as_value_iter();
        if let Some(value) = values.next() {
            <&T as InsertableQueryfragment<Tab, Sqlite>>::walk_ast_helper_with_value_clause(
                value,
                out.reborrow(),
            )?;
        }
        for value in values {
            out.push_sql(", (");
            <&T as InsertableQueryfragment<Tab, Sqlite>>::walk_ast_helper_without_value_clause(
                value,
                out.reborrow(),
            )?;
            out.push_sql(")");
        }
        Ok(())
    }
}

#[allow(missing_copy_implementations, missing_debug_implementations)]
#[repr(transparent)]
pub struct SqliteCanInsertInSingleQueryHelper<T: ?Sized>(T);

impl<V, T, QId, const STATIC_QUERY_ID: bool> CanInsertInSingleQuery<Sqlite>
    for SqliteBatchInsertWrapper<V, T, QId, STATIC_QUERY_ID>
where
    // We constrain that here on an internal helper type
    // to make sure that this does not accidently leak
    // so that noone does really implement normal batch
    // insert for inserts with default values here
    SqliteCanInsertInSingleQueryHelper<V>: CanInsertInSingleQuery<Sqlite>,
    V: AsValueIterator<T>,
    V::Item: Insertable<T>,
    <V::Item as Insertable<T>>::Values: ContainsDefaultableValue<Out = No>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        let values = &self.0.values;
        let values = unsafe {
            // This cast is safe as `SqliteCanInsertInSingleQueryHelper` is #[repr(transparent)]
            &*(values as *const V as *const SqliteCanInsertInSingleQueryHelper<V>)
        };
        values.rows_to_insert()
    }
}

impl<T, const N: usize> CanInsertInSingleQuery<Sqlite>
    for SqliteCanInsertInSingleQueryHelper<[T; N]>
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(N)
    }
}

impl<T, const N: usize> CanInsertInSingleQuery<Sqlite>
    for SqliteCanInsertInSingleQueryHelper<Box<[T; N]>>
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(N)
    }
}

impl<T> CanInsertInSingleQuery<Sqlite> for SqliteCanInsertInSingleQueryHelper<[T]> {
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

impl<'a, T> CanInsertInSingleQuery<Sqlite> for SqliteCanInsertInSingleQueryHelper<&'a [T]> {
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

impl<T> CanInsertInSingleQuery<Sqlite> for SqliteCanInsertInSingleQueryHelper<Vec<T>> {
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

impl<T> CanInsertInSingleQuery<Sqlite> for SqliteCanInsertInSingleQueryHelper<T>
where
    T: CanInsertInSingleQuery<Sqlite>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        self.0.rows_to_insert()
    }
}

impl<V, T, QId, const STATIC_QUERY_ID: bool> QueryId
    for SqliteBatchInsertWrapper<V, T, QId, STATIC_QUERY_ID>
where
    BatchInsert<V, T, QId, STATIC_QUERY_ID>: QueryId,
{
    type QueryId = <BatchInsert<V, T, QId, STATIC_QUERY_ID> as QueryId>::QueryId;

    const HAS_STATIC_QUERY_ID: bool =
        <BatchInsert<V, T, QId, STATIC_QUERY_ID> as QueryId>::HAS_STATIC_QUERY_ID;
}

impl<V, T, QId, C, Op, const STATIC_QUERY_ID: bool> ExecuteDsl<C, Sqlite>
    for (
        No,
        InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>,
    )
where
    C: Connection<Backend = Sqlite>,
    T: Table + QueryId,
    T::FromClause: QueryFragment<Sqlite>,
    Op: QueryFragment<Sqlite> + QueryId,
    SqliteBatchInsertWrapper<V, T, QId, STATIC_QUERY_ID>:
        QueryFragment<Sqlite> + QueryId + CanInsertInSingleQuery<Sqlite>,
{
    fn execute((No, query): Self, conn: &mut C) -> QueryResult<usize> {
        let query = InsertStatement {
            records: SqliteBatchInsertWrapper(query.records),
            operator: query.operator,
            target: query.target,
            returning: query.returning,
        };
        query.execute(conn)
    }
}

macro_rules! tuple_impls {
        ($(
            $Tuple:tt {
                $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
            }
        )+) => {
            $(
                impl_contains_defaultable_value!($($T,)*);
            )*
        }
    }

macro_rules! impl_contains_defaultable_value {
    (
        @build
        start_ts = [$($ST: ident,)*],
        ts = [$T1: ident,],
        bounds = [$($bounds: tt)*],
        out = [$($out: tt)*],
    )=> {
        impl<$($ST,)*> ContainsDefaultableValue for ($($ST,)*)
        where
            $($ST: ContainsDefaultableValue,)*
            $($bounds)*
            $T1::Out: Any<$($out)*>,
        {
            type Out = <$T1::Out as Any<$($out)*>>::Out;
        }

    };
    (
        @build
        start_ts = [$($ST: ident,)*],
        ts = [$T1: ident, $($T: ident,)+],
        bounds = [$($bounds: tt)*],
        out = [$($out: tt)*],
    )=> {
        impl_contains_defaultable_value! {
            @build
            start_ts = [$($ST,)*],
            ts = [$($T,)*],
            bounds = [$($bounds)* $T1::Out: Any<$($out)*>,],
            out = [<$T1::Out as Any<$($out)*>>::Out],
        }
    };
    ($T1: ident, $($T: ident,)+) => {
        impl_contains_defaultable_value! {
            @build
            start_ts = [$T1, $($T,)*],
            ts = [$($T,)*],
            bounds = [],
            out = [$T1::Out],
        }
    };
    ($T1: ident,) => {
        impl<$T1> ContainsDefaultableValue for ($T1,)
        where $T1: ContainsDefaultableValue,
        {
            type Out = <$T1 as ContainsDefaultableValue>::Out;
        }
    }
}

diesel_derives::__diesel_for_each_tuple!(tuple_impls);
