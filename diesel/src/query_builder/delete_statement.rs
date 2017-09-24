use backend::Backend;
use expression::{AppearsOnTable, SelectableExpression};
use query_builder::*;
use query_builder::returning_clause::*;
use query_builder::where_clause::*;
use query_source::Table;
use result::QueryResult;

#[derive(Debug)]
pub struct DeleteStatement<T, U, Ret = NoReturningClause> {
    table: T,
    where_clause: U,
    returning: Ret,
}

impl<T, U> DeleteStatement<T, U, NoReturningClause> {
    #[doc(hidden)]
    pub fn new(table: T, where_clause: U) -> Self {
        DeleteStatement {
            table: table,
            where_clause: where_clause,
            returning: NoReturningClause,
        }
    }
}

impl<T, U, Ret> DeleteStatement<T, U, Ret> {
    /// Adds the given predicate to the `WHERE` clause of the statement being
    /// constructed.
    ///
    /// If there is already a `WHERE` clause, the predicate will be appended
    /// with `AND`. There is no difference in behavior between
    /// `delete(table.filter(x))` and `delete(table).filter(x)`.
    ///
    /// # Example
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
    /// # fn main() {
    /// #     use users::dsl::*;
    /// #     let connection = establish_connection();
    /// let deleted_rows = diesel::delete(users)
    ///     .filter(name.eq("Sean"))
    ///     .execute(&connection);
    /// assert_eq!(Ok(1), deleted_rows);
    ///
    /// let expected_names = vec!["Tess".to_string()];
    /// let names = users.select(name).load(&connection);
    ///
    /// assert_eq!(Ok(expected_names), names);
    /// # }
    /// ```
    pub fn filter<V>(self, predicate: V) -> DeleteStatement<T, U::Output, Ret>
    where
        U: WhereAnd<V>,
        V: AppearsOnTable<T>,
    {
        DeleteStatement {
            table: self.table,
            where_clause: self.where_clause.and(predicate),
            returning: self.returning,
        }
    }
}

impl<T, U, Ret, DB> QueryFragment<DB> for DeleteStatement<T, U, Ret>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_sql("DELETE FROM ");
        self.table.from_clause().walk_ast(out.reborrow())?;
        self.where_clause.walk_ast(out.reborrow())?;
        self.returning.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl_query_id!(DeleteStatement<T, U, Ret>);

impl<T, U> AsQuery for DeleteStatement<T, U, NoReturningClause>
where
    T: Table,
    T::AllColumns: SelectableExpression<T>,
    DeleteStatement<T, U, ReturningClause<T::AllColumns>>: Query,
{
    type SqlType = <Self::Query as Query>::SqlType;
    type Query = DeleteStatement<T, U, ReturningClause<T::AllColumns>>;

    fn as_query(self) -> Self::Query {
        self.returning(T::all_columns())
    }
}

impl<T, U, Ret> Query for DeleteStatement<T, U, ReturningClause<Ret>>
where
    T: Table,
    Ret: SelectableExpression<T>,
{
    type SqlType = Ret::SqlType;
}

impl<T, U> DeleteStatement<T, U, NoReturningClause> {
    /// Specify what expression is returned after execution of the `delete`.
    ///
    /// # Examples
    ///
    /// ### Deleting a record:
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
    /// let deleted_name = diesel::delete(users.filter(name.eq("Sean")))
    ///     .returning(name)
    ///     .get_result(&connection);
    /// assert_eq!(Ok("Sean".to_string()), deleted_name);
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn main() {}
    /// ```
    pub fn returning<E>(self, returns: E) -> DeleteStatement<T, U, ReturningClause<E>>
    where
        E: SelectableExpression<T>,
        DeleteStatement<T, U, ReturningClause<E>>: Query,
    {
        DeleteStatement {
            table: self.table,
            where_clause: self.where_clause,
            returning: ReturningClause(returns),
        }
    }
}
