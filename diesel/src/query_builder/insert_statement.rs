use backend::Backend;
use expression::{Expression, NonAggregate, SelectableExpression};
use expression::operators::Eq;
use insertable::{CanInsertInSingleQuery, InsertValues, Insertable};
use query_builder::*;
#[cfg(feature = "sqlite")]
use query_dsl::ExecuteDsl;
use query_source::{Column, Table};
use result::QueryResult;
#[cfg(feature = "sqlite")]
use sqlite::{Sqlite, SqliteConnection};
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
    pub fn into<S>(self, target: S) -> InsertStatement<S, T, Op> {
        InsertStatement::new(target, self.records, self.operator, NoReturningClause)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InsertStatement<T, U, Op = Insert, Ret = NoReturningClause> {
    operator: Op,
    target: T,
    records: U,
    returning: Ret,
}

impl<T, U, Op, Ret> InsertStatement<T, U, Op, Ret> {
    fn new(target: T, records: U, operator: Op, returning: Ret) -> Self {
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
    U: Insertable<T, DB> + CanInsertInSingleQuery<DB> + Copy,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();

        if self.records.rows_to_insert() == 0 {
            out.push_sql("SELECT 1 FROM ");
            self.target.from_clause().walk_ast(out.reborrow())?;
            out.push_sql(" WHERE 1=0");
            return Ok(());
        }

        let values = self.records.values();
        self.operator.walk_ast(out.reborrow())?;
        out.push_sql(" INTO ");
        self.target.from_clause().walk_ast(out.reborrow())?;
        if self.records.values().is_noop() {
            out.push_sql(" DEFAULT VALUES");
        } else {
            out.push_sql(" (");
            if let Some(builder) = out.reborrow().query_builder() {
                values.column_names(builder)?;
            }
            out.push_sql(") VALUES ");
            self.records.values().walk_ast(out.reborrow())?;
        }
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<'a, T, U, Op> ExecuteDsl<SqliteConnection> for InsertStatement<T, &'a [U], Op>
where
    InsertStatement<T, &'a U, Op>: QueryFragment<Sqlite>,
    T: Copy,
    Op: Copy,
{
    fn execute(self, conn: &SqliteConnection) -> QueryResult<usize> {
        let mut result = 0;
        for record in self.records {
            result += InsertStatement::new(self.target, record, self.operator, self.returning)
                .execute(conn)?;
        }
        Ok(result)
    }
}

#[cfg(feature = "sqlite")]
impl<'a, T, U, Op> ExecuteDsl<SqliteConnection> for InsertStatement<T, &'a Vec<U>, Op>
where
    InsertStatement<T, &'a [U], Op>: ExecuteDsl<SqliteConnection>,
{
    fn execute(self, conn: &SqliteConnection) -> QueryResult<usize> {
        InsertStatement::new(
            self.target,
            self.records.as_slice(),
            self.operator,
            self.returning,
        ).execute(conn)
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

impl<T, U, Op> InsertStatement<T, U, Op> {
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
    pub fn returning<E>(self, returns: E) -> InsertStatement<T, U, Op, ReturningClause<E>>
    where
        InsertStatement<T, U, Op, ReturningClause<E>>: Query,
    {
        InsertStatement::new(
            self.target,
            self.records,
            self.operator,
            ReturningClause(returns),
        )
    }
}

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

impl<'a, Lhs, Rhs> UndecoratedInsertRecord<Lhs::Table> for &'a Eq<Lhs, Rhs>
where
    Lhs: Column,
{
}

impl<'a, Lhs, Rhs, Tab> UndecoratedInsertRecord<Tab> for &'a Option<Eq<Lhs, Rhs>>
where
    &'a Eq<Lhs, Rhs>: UndecoratedInsertRecord<Tab>,
{
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct DefaultValues;

impl<DB: Backend> CanInsertInSingleQuery<DB> for DefaultValues {
    fn rows_to_insert(&self) -> usize {
        1
    }
}

impl<'a, Tab, DB> Insertable<Tab, DB> for &'a DefaultValues
where
    Tab: Table,
    DB: Backend,
{
    type Values = DefaultValues;

    fn values(self) -> Self::Values {
        *self
    }
}

impl<Tab, DB> InsertValues<Tab, DB> for DefaultValues
where
    Tab: Table,
    DB: Backend,
{
    fn column_names(&self, _: &mut DB::QueryBuilder) -> QueryResult<()> {
        Ok(())
    }

    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }

    fn is_noop(&self) -> bool {
        true
    }
}
