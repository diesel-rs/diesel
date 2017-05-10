use backend::Backend;
use expression::SelectableExpression;
use query_builder::*;
use query_builder::returning_clause::*;
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

impl<T, U, Ret, DB> QueryFragment<DB> for DeleteStatement<T, U, Ret> where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
{
    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        out.push_sql("DELETE FROM ");
        try!(self.table.from_clause().to_sql(out));
        try!(self.where_clause.to_sql(out));
        try!(self.returning.to_sql(out));
        Ok(())
    }

    fn walk_ast(&self, mut pass: AstPass<DB>) -> QueryResult<()> {
        self.table.from_clause().walk_ast(pass.reborrow())?;
        self.where_clause.walk_ast(pass.reborrow())?;
        self.returning.walk_ast(pass.reborrow())?;
        Ok(())
    }
}

impl_query_id!(DeleteStatement<T, U, Ret>);

impl<T, U> AsQuery for DeleteStatement<T, U, NoReturningClause> where
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

impl<T, U, Ret> Query for DeleteStatement<T, U, ReturningClause<Ret>> where
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
    /// # include!("src/doctest_setup.rs");
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
    pub fn returning<E>(self, returns: E) -> DeleteStatement<T, U, ReturningClause<E>> where
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
