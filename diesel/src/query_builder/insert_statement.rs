use backend::Backend;
use connection::Connection;
use expression::{Expression, NonAggregate, SelectableExpression};
use insertable::{InsertValues, Insertable};
use query_builder::*;
use query_dsl::{ExecuteDsl, LoadDsl, LoadQuery};
use query_source::Table;
use result::QueryResult;
use super::returning_clause::*;

/// The structure returned by [`insert`](/diesel/fn.insert.html). The only thing that can be done with it
/// is call `into`.
#[derive(Debug, Copy, Clone)]
pub struct IncompleteInsertStatement<T, Op> {
    records: T,
    operator: Op,
}

impl<T, Op> IncompleteInsertStatement<T, Op> {
    #[doc(hidden)]
    pub fn new(records: T, operator: Op) -> Self {
        IncompleteInsertStatement {
            records: records,
            operator: operator,
        }
    }

    /// Specify which table the data passed to `insert` should be added to.
    pub fn into<S>(self, target: S) -> T::InsertStatement
    where
        T: IntoInsertStatement<S, Op>,
    {
        self.records.into_insert_statement(target, self.operator)
    }
}

pub trait IntoInsertStatement<Tab, Op> {
    type InsertStatement;

    fn into_insert_statement(self, target: Tab, operator: Op) -> Self::InsertStatement;
}

impl<'a, T, Tab, Op> IntoInsertStatement<Tab, Op> for &'a [T]
where
    &'a T: UndecoratedInsertRecord<Tab>,
{
    type InsertStatement = BatchInsertStatement<Tab, Self, Op, NoReturningClause>;

    fn into_insert_statement(self, target: Tab, operator: Op) -> Self::InsertStatement {
        BatchInsertStatement {
            operator: operator,
            target: target,
            records: self,
            returning: NoReturningClause,
        }
    }
}

impl<'a, T, Tab, Op> IntoInsertStatement<Tab, Op> for &'a Vec<T>
where
    &'a [T]: IntoInsertStatement<Tab, Op>,
{
    type InsertStatement = <&'a [T] as IntoInsertStatement<Tab, Op>>::InsertStatement;

    fn into_insert_statement(self, target: Tab, operator: Op) -> Self::InsertStatement {
        (&**self).into_insert_statement(target, operator)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InsertStatement<T, U, Op = Insert, Ret = NoReturningClause> {
    operator: Op,
    target: T,
    records: U,
    returning: Ret,
}

impl<T, U, Op> InsertStatement<T, U, Op> {
    pub fn no_returning_clause(target: T, records: U, operator: Op) -> Self {
        InsertStatement::new(target, records, operator, NoReturningClause)
    }
}

impl<T, U, Op, Ret> InsertStatement<T, U, Op, Ret> {
    pub fn new(target: T, records: U, operator: Op, returning: Ret) -> Self {
        InsertStatement {
            operator: operator,
            target: target,
            records: records,
            returning: returning,
        }
    }
}

impl<T, U, Op, Ret, DB> QueryFragment<DB> for InsertStatement<T, U, Op, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: Insertable<T, DB> + Copy,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        let values = self.records.values();
        out.unsafe_to_cache_prepared();
        self.operator.walk_ast(out.reborrow())?;
        out.push_sql(" INTO ");
        self.target.from_clause().walk_ast(out.reborrow())?;
        out.push_sql(" (");
        if let Some(builder) = out.reborrow().query_builder() {
            values.column_names(builder)?;
        }
        out.push_sql(") VALUES ");
        self.records.values().walk_ast(out.reborrow())?;
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl_query_id!(noop: InsertStatement<T, U, Op, Ret>);

impl<T, U, Op> AsQuery for InsertStatement<T, U, Op, NoReturningClause>
where
    T: Table,
    InsertStatement<T, U, Op, ReturningClause<T::AllColumns>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = InsertStatement<T, U, Op, ReturningClause<T::AllColumns>>;

    fn as_query(self) -> Self::Query {
        self.returning(T::all_columns())
    }
}

impl<T, U, Op, Ret> Query for InsertStatement<T, U, Op, ReturningClause<Ret>>
where
    Ret: Expression + SelectableExpression<T> + NonAggregate,
{
    type SqlType = Ret::SqlType;
}

impl<T, U, Op> InsertStatement<T, U, Op, NoReturningClause> {
    /// Specify what expression is returned after execution of the `insert`.
    /// This method can only be called once.
    ///
    /// # Examples
    ///
    /// ### Inserting a record:
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let new_user = NewUser {
    ///     name: "Timmy".to_string(),
    /// };
    ///
    /// let inserted_name = diesel::insert(&new_user)
    ///     .into(users)
    ///     .returning(name)
    ///     .get_result(&connection);
    /// assert_eq!(Ok("Timmy".to_string()), inserted_name);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> InsertStatement<T, U, Op, ReturningClause<E>>
    where
        InsertStatement<T, U, Op, ReturningClause<E>>: Query,
    {
        InsertStatement {
            operator: self.operator,
            target: self.target,
            records: self.records,
            returning: ReturningClause(returns),
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// The result of calling `insert(records).into(some_table)` when `records` is
/// a slice or a `Vec`. When calling methods from `ExecuteDsl` or `LoadDsl`.
/// When the given slice is empty, this struct will not execute any queries.
/// When the given slice is not empty, this will execute a single bulk insert
/// on backends which support the `DEFAULT` keyword, and one query per record
/// on backends which do not (SQLite).
pub struct BatchInsertStatement<T, U, Op = Insert, Ret = NoReturningClause> {
    operator: Op,
    target: T,
    records: U,
    returning: Ret,
}

impl<T, U, Op, Ret> BatchInsertStatement<T, U, Op, Ret> {
    fn into_insert_statement(self) -> InsertStatement<T, U, Op, Ret> {
        InsertStatement::new(self.target, self.records, self.operator, self.returning)
    }
}

impl<T, U, Op> BatchInsertStatement<T, U, Op> {
    /// Specify what expression is returned after execution of the `insert`.
    /// # Examples
    ///
    /// ### Inserting records:
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let new_users = vec![
    ///     NewUser { name: "Timmy".to_string(), },
    ///     NewUser { name: "Jimmy".to_string(), },
    /// ];
    ///
    /// let inserted_names = diesel::insert(&new_users)
    ///     .into(users)
    ///     .returning(name)
    ///     .get_results(&connection);
    /// assert_eq!(Ok(vec!["Timmy".to_string(), "Jimmy".to_string()]), inserted_names);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> BatchInsertStatement<T, U, Op, ReturningClause<E>> {
        BatchInsertStatement {
            operator: self.operator,
            target: self.target,
            records: self.records,
            returning: ReturningClause(returns),
        }
    }
}

impl<'a, T, U, Op, Ret, Conn, DB> ExecuteDsl<Conn, DB> for BatchInsertStatement<T, &'a [U], Op, Ret>
where
    Conn: Connection<Backend = DB>,
    DB: Backend,
    InsertStatement<T, &'a [U], Op, Ret>: ExecuteDsl<Conn>,
{
    fn execute(self, conn: &Conn) -> QueryResult<usize> {
        if self.records.is_empty() {
            Ok(0)
        } else {
            self.into_insert_statement().execute(conn)
        }
    }
}

impl<'a, T, U, V, Op, Ret, Conn> LoadQuery<Conn, V> for BatchInsertStatement<T, &'a [U], Op, Ret>
where
    InsertStatement<T, &'a [U], Op, Ret>: LoadQuery<Conn, V>,
{
    fn internal_load(self, conn: &Conn) -> QueryResult<Vec<V>> {
        if self.records.is_empty() {
            Ok(Vec::new())
        } else {
            self.into_insert_statement().internal_load(conn)
        }
    }
}

impl<'a, T, U, Op, Ret, Conn> LoadDsl<Conn> for BatchInsertStatement<T, &'a [U], Op, Ret> {}

#[derive(Debug, Copy, Clone)]
pub struct Insert;

impl<DB: Backend> QueryFragment<DB> for Insert {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("INSERT");
        Ok(())
    }
}

impl_query_id!(Insert);

/// Marker trait to indicate that no additional operations have been added
/// to a record for insert. Used to prevent things like
/// `insert(&vec![user.on_conflict_do_nothing(), user2.on_conflict_do_nothing()])`
/// from compiling.
pub trait UndecoratedInsertRecord<Table> {}

impl<'a, T, Table> UndecoratedInsertRecord<Table> for &'a [T]
where
    &'a T: UndecoratedInsertRecord<Table>,
{
}

impl<'a, T, Table> UndecoratedInsertRecord<Table> for &'a Vec<T>
where
    &'a [T]: UndecoratedInsertRecord<Table>,
{
}

/// The structure returned by [`insert_default_values`](/diesel/fn.insert_default_values.html). The
/// only thing that can be done with it is call `into`.
#[derive(Debug, Copy, Clone, Default)]
pub struct IncompleteDefaultInsertStatement {}

#[derive(Debug, Copy, Clone)]
pub struct DefaultInsertStatement<T, Ret = NoReturningClause> {
    target: T,
    returning: Ret,
}

impl IncompleteDefaultInsertStatement {
    #[doc(hidden)]
    pub fn new() -> Self {
        IncompleteDefaultInsertStatement {}
    }

    /// Specify which table the data passed to `insert` should be added to.
    pub fn into<S>(self, target: S) -> DefaultInsertStatement<S> {
        DefaultInsertStatement {
            target: target,
            returning: NoReturningClause,
        }
    }
}

impl<T> DefaultInsertStatement<T, NoReturningClause> {
    /// Specify what expression is returned after execution of the `insert_default_values`.
    /// This method can only be called once.
    pub fn returning<E>(self, returns: E) -> DefaultInsertStatement<T, ReturningClause<E>>
    where
        DefaultInsertStatement<T, ReturningClause<E>>: Query,
    {
        DefaultInsertStatement {
            target: self.target,
            returning: ReturningClause(returns),
        }
    }
}

impl<T, Ret> Query for DefaultInsertStatement<T, ReturningClause<Ret>>
where
    Ret: Expression + SelectableExpression<T> + NonAggregate,
{
    type SqlType = Ret::SqlType;
}

impl<T, Ret, DB> QueryFragment<DB> for DefaultInsertStatement<T, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        out.push_sql("INSERT INTO ");
        self.target.from_clause().walk_ast(out.reborrow())?;
        out.push_sql("DEFAULT VALUES ");
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl_query_id!(noop: DefaultInsertStatement<T, Ret>);
