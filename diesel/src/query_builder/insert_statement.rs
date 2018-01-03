use std::any::*;

use backend::Backend;
use expression::{Expression, NonAggregate, SelectableExpression};
use expression::operators::Eq;
use insertable::*;
#[cfg(feature = "mysql")]
use mysql::Mysql;
use query_builder::*;
#[cfg(feature = "sqlite")]
use query_dsl::methods::ExecuteDsl;
use query_dsl::RunQueryDsl;
use query_source::{Column, Table};
use result::QueryResult;
#[cfg(feature = "sqlite")]
use sqlite::{Sqlite, SqliteConnection};
use super::returning_clause::*;

/// The structure returned by [`insert_into`].
///
/// The provided methods [`values`] and [`default_values`] will insert
/// data into the targeted table.
///
/// [`insert_into`]: ../fn.insert_into.html
/// [`values`]: #method.values
/// [`default_values`]: #method.default_values
#[derive(Debug, Clone, Copy)]
pub struct IncompleteInsertStatement<T, Op> {
    target: T,
    operator: Op,
}

impl<T, Op> IncompleteInsertStatement<T, Op> {
    pub(crate) fn new(target: T, operator: Op) -> Self {
        IncompleteInsertStatement { target, operator }
    }

    /// Inserts `DEFAULT VALUES` into the targeted table.
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users (name) {
    /// #         name -> Text,
    /// #         hair_color -> Text,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use users::dsl::*;
    /// #     let connection = connection_no_data();
    /// connection.execute("CREATE TABLE users (
    ///     name VARCHAR(255) NOT NULL DEFAULT 'Sean',
    ///     hair_color VARCHAR(255) NOT NULL DEFAULT 'Green'
    /// )")?;
    ///
    /// insert_into(users)
    ///     .default_values()
    ///     .execute(&connection)
    ///     .unwrap();
    /// let inserted_user = users.first(&connection)?;
    /// let expected_data = (String::from("Sean"), String::from("Green"));
    ///
    /// assert_eq!(expected_data, inserted_user);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn default_values(self) -> InsertStatement<T, DefaultValues, Op> {
        static STATIC_DEFAULT_VALUES: &DefaultValues = &DefaultValues;
        self.values(STATIC_DEFAULT_VALUES)
    }

    /// Inserts the given values into the table passed to `insert_into`.
    ///
    /// See the documentation of [`insert_into`] for
    /// usage examples.
    ///
    /// This method can sometimes produce extremely opaque error messages due to
    /// limitations of the Rust language. If you receive an error about
    /// "overflow evaluating requirement" as a result of calling this method,
    /// you may need an `&` in front of the argument to this method.
    ///
    /// [`insert_into`]: ../fn.insert_into.html
    pub fn values<U>(self, records: U) -> InsertStatement<T, U::Values, Op>
    where
        U: Insertable<T>,
    {
        InsertStatement::new(
            self.target,
            records.values(),
            self.operator,
            NoReturningClause,
        )
    }
}

#[derive(Debug, Copy, Clone)]
/// A fully constructed insert statement.
///
/// The parameters of this struct represent:
///
/// - `T`: The table we are inserting into
/// - `U`: The data being inserted
/// - `Op`: The operation being performed. The specific types used to represent
///   this are private, but correspond to SQL such as `INSERT` or `REPLACE`.
///   You can safely rely on the default type representing `INSERT`
/// - `Ret`: The `RETURNING` clause of the query. The specific types used to
///   represent this are private. You can safely rely on the default type
///   representing a query without a `RETURNING` clause.
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

    #[cfg(feature = "postgres")]
    pub(crate) fn replace_values<F, V>(self, f: F) -> InsertStatement<T, V, Op, Ret>
    where
        F: FnOnce(U) -> V,
    {
        InsertStatement::new(self.target, f(self.records), self.operator, self.returning)
    }
}

impl<T, U, Op, Ret, DB> QueryFragment<DB> for InsertStatement<T, U, Op, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: InsertValues<T, DB> + CanInsertInSingleQuery<DB>,
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

        self.operator.walk_ast(out.reborrow())?;
        out.push_sql(" INTO ");
        self.target.from_clause().walk_ast(out.reborrow())?;
        if self.records.is_noop() {
            out.push_sql(" DEFAULT VALUES");
        } else {
            out.push_sql(" (");
            if let Some(builder) = out.reborrow().query_builder() {
                self.records.column_names(builder)?;
            }
            out.push_sql(") VALUES ");
            if self.records.requires_parenthesis() {
                out.push_sql("(");
            }
            self.records.walk_ast(out.reborrow())?;
            if self.records.requires_parenthesis() {
                out.push_sql(")");
            }
        }
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<'a, T, U, Op> ExecuteDsl<SqliteConnection> for InsertStatement<T, &'a [U], Op>
where
    &'a U: Insertable<T>,
    InsertStatement<T, <&'a U as Insertable<T>>::Values, Op>: QueryFragment<Sqlite>,
    T: Copy,
    Op: Copy,
{
    fn execute(query: Self, conn: &SqliteConnection) -> QueryResult<usize> {
        use connection::Connection;
        conn.transaction(|| {
            let mut result = 0;
            for record in query.records {
                result += InsertStatement::new(
                    query.target,
                    record.values(),
                    query.operator,
                    query.returning,
                ).execute(conn)?;
            }
            Ok(result)
        })
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

impl<T, U, Op, Ret, Conn> RunQueryDsl<Conn> for InsertStatement<T, U, Op, Ret> {}

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
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let inserted_names = diesel::insert_into(users)
    ///     .values(&vec![name.eq("Timmy"), name.eq("Jimmy")])
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
#[doc(hidden)]
pub struct Insert;

impl<DB: Backend> QueryFragment<DB> for Insert {
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("INSERT");
        Ok(())
    }
}

impl_query_id!(Insert);

#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub struct Replace;

#[cfg(feature = "sqlite")]
impl QueryFragment<Sqlite> for Replace {
    fn walk_ast(&self, mut out: AstPass<Sqlite>) -> QueryResult<()> {
        out.push_sql("REPLACE");
        Ok(())
    }
}

#[cfg(feature = "mysql")]
impl QueryFragment<Mysql> for Replace {
    fn walk_ast(&self, mut out: AstPass<Mysql>) -> QueryResult<()> {
        out.push_sql("REPLACE");
        Ok(())
    }
}

impl_query_id!(Replace);

/// Marker trait to indicate that no additional operations have been added
/// to a record for insert.
///
/// This is used to prevent things like
/// `.on_conflict_do_nothing().on_conflict_do_nothing()`
/// from compiling.
pub trait UndecoratedInsertRecord<Table> {}

impl<'a, T, Tab> UndecoratedInsertRecord<Tab> for &'a T
where
    T: ?Sized + UndecoratedInsertRecord<Tab>,
{
}

impl<T, U> UndecoratedInsertRecord<T::Table> for ColumnInsertValue<T, U>
where
    T: Column,
{
}

impl<T, Table> UndecoratedInsertRecord<Table> for [T]
where
    T: UndecoratedInsertRecord<Table>,
{
}

impl<T, Table> UndecoratedInsertRecord<Table> for Vec<T>
where
    [T]: UndecoratedInsertRecord<Table>,
{
}

impl<Lhs, Rhs> UndecoratedInsertRecord<Lhs::Table> for Eq<Lhs, Rhs>
where
    Lhs: Column,
{
}

impl<Lhs, Rhs, Tab> UndecoratedInsertRecord<Tab> for Option<Eq<Lhs, Rhs>>
where
    Eq<Lhs, Rhs>: UndecoratedInsertRecord<Tab>,
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

impl<'a, Tab> Insertable<Tab> for &'a DefaultValues {
    type Values = DefaultValues;

    fn values(self) -> Self::Values {
        *self
    }
}

impl<Tab, DB> InsertValues<Tab, DB> for DefaultValues
where
    Tab: Table,
    DB: Backend + Any,
{
    fn column_names(&self, _: &mut DB::QueryBuilder) -> QueryResult<()> {
        Ok(())
    }

    fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
        Ok(())
    }

    #[cfg(not(feature = "mysql"))]
    fn is_noop(&self) -> bool {
        true
    }

    #[cfg(feature = "mysql")]
    fn is_noop(&self) -> bool {
        // The syntax for this on MySQL is
        // INSERT INTO table () VALUES ()
        //
        // This is hacky, but it's the easiest way to get this done without a
        // deeper restructuring of this code.
        TypeId::of::<DB>() != TypeId::of::<::mysql::Mysql>()
    }
}
