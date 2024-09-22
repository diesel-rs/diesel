use super::{BatchInsert, InsertStatement};
use crate::insertable::InsertValues;
use crate::insertable::{CanInsertInSingleQuery, ColumnInsertValue, DefaultableColumnInsertValue};
use crate::prelude::*;
use crate::query_builder::{AstPass, QueryId, ValuesClause};
use crate::query_builder::{DebugQuery, QueryFragment};
use crate::query_dsl::methods::ExecuteDsl;
use crate::sqlite::Sqlite;
use std::fmt::{self, Debug, Display};

pub trait DebugQueryHelper<ContainsDefaultableValue> {
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn fmt_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<'a, T, V, QId, Op, Ret, const STATIC_QUERY_ID: bool> DebugQueryHelper<Yes>
    for DebugQuery<
        'a,
        InsertStatement<T, BatchInsert<Vec<ValuesClause<V, T>>, T, QId, STATIC_QUERY_ID>, Op, Ret>,
        Sqlite,
    >
where
    V: QueryFragment<Sqlite>,
    T: Copy + QuerySource,
    Op: Copy,
    Ret: Copy,
    for<'b> InsertStatement<T, &'b ValuesClause<V, T>, Op, Ret>: QueryFragment<Sqlite>,
{
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut statements = vec![String::from("BEGIN")];
        for record in self.query.records.values.iter() {
            let stmt = InsertStatement::new(
                self.query.target,
                record,
                self.query.operator,
                self.query.returning,
            );
            statements.push(crate::debug_query(&stmt).to_string());
        }
        statements.push("COMMIT".into());

        f.debug_struct("Query")
            .field("sql", &statements)
            .field("binds", &[] as &[i32; 0])
            .finish()
    }

    fn fmt_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BEGIN;")?;
        for record in self.query.records.values.iter() {
            let stmt = InsertStatement::new(
                self.query.target,
                record,
                self.query.operator,
                self.query.returning,
            );
            writeln!(f, "{}", crate::debug_query(&stmt))?;
        }
        writeln!(f, "COMMIT;")?;
        Ok(())
    }
}

#[allow(unsafe_code)] // cast to transparent wrapper type
impl<'a, T, V, QId, Op, const STATIC_QUERY_ID: bool> DebugQueryHelper<No>
    for DebugQuery<'a, InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>, Sqlite>
where
    T: Copy + QuerySource,
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
                'a,
                InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>,
                Sqlite,
            >
                as *const DebugQuery<
                    'a,
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
                'a,
                InsertStatement<T, BatchInsert<V, T, QId, STATIC_QUERY_ID>, Op>,
                Sqlite,
            >
                as *const DebugQuery<
                    'a,
                    InsertStatement<T, SqliteBatchInsertWrapper<V, T, QId, STATIC_QUERY_ID>, Op>,
                    Sqlite,
                >)
        };
        <_ as Display>::fmt(value, f)
    }
}

impl<'a, T, V, QId, Op, O, const STATIC_QUERY_ID: bool> Display
    for DebugQuery<
        'a,
        InsertStatement<T, BatchInsert<Vec<ValuesClause<V, T>>, T, QId, STATIC_QUERY_ID>, Op>,
        Sqlite,
    >
where
    T: QuerySource,
    V: ContainsDefaultableValue<Out = O>,
    Self: DebugQueryHelper<O>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_display(f)
    }
}

impl<'a, T, V, QId, Op, O, const STATIC_QUERY_ID: bool> Debug
    for DebugQuery<
        'a,
        InsertStatement<T, BatchInsert<Vec<ValuesClause<V, T>>, T, QId, STATIC_QUERY_ID>, Op>,
        Sqlite,
    >
where
    T: QuerySource,
    V: ContainsDefaultableValue<Out = O>,
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
    for InsertStatement<T, BatchInsert<Vec<ValuesClause<V, T>>, T, QId, STATIC_QUERY_ID>, Op>
where
    T: QuerySource,
    C: Connection<Backend = Sqlite>,
    V: ContainsDefaultableValue<Out = O>,
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
        InsertStatement<T, BatchInsert<Vec<ValuesClause<V, T>>, T, QId, STATIC_QUERY_ID>, Op>,
    )
where
    C: Connection<Backend = Sqlite>,
    T: Table + Copy + QueryId + 'static,
    T::FromClause: QueryFragment<Sqlite>,
    Op: Copy + QueryId + QueryFragment<Sqlite>,
    V: InsertValues<Sqlite, T> + CanInsertInSingleQuery<Sqlite> + QueryId,
{
    fn execute((Yes, query): Self, conn: &mut C) -> QueryResult<usize> {
        conn.transaction(|conn| {
            let mut result = 0;
            for record in &query.records.values {
                let stmt =
                    InsertStatement::new(query.target, record, query.operator, query.returning);
                result += stmt.execute(conn)?;
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

impl<V, Tab, QId, const STATIC_QUERY_ID: bool> QueryFragment<Sqlite>
    for SqliteBatchInsertWrapper<Vec<ValuesClause<V, Tab>>, Tab, QId, STATIC_QUERY_ID>
where
    ValuesClause<V, Tab>: QueryFragment<Sqlite>,
    V: QueryFragment<Sqlite>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        if !STATIC_QUERY_ID {
            out.unsafe_to_cache_prepared();
        }

        let mut values = self.0.values.iter();
        if let Some(value) = values.next() {
            value.walk_ast(out.reborrow())?;
        }
        for value in values {
            out.push_sql(", (");
            value.values.walk_ast(out.reborrow())?;
            out.push_sql(")");
        }
        Ok(())
    }
}

#[allow(missing_copy_implementations, missing_debug_implementations)]
#[repr(transparent)]
pub struct SqliteCanInsertInSingleQueryHelper<T: ?Sized>(T);

impl<V, T, QId, const STATIC_QUERY_ID: bool> CanInsertInSingleQuery<Sqlite>
    for SqliteBatchInsertWrapper<Vec<ValuesClause<V, T>>, T, QId, STATIC_QUERY_ID>
where
    // We constrain that here on an internal helper type
    // to make sure that this does not accidentally leak
    // so that none does really implement normal batch
    // insert for inserts with default values here
    SqliteCanInsertInSingleQueryHelper<V>: CanInsertInSingleQuery<Sqlite>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(self.0.values.len())
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
    T: Table + QueryId + 'static,
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
            into_clause: query.into_clause,
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
