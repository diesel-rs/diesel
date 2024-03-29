use std::marker::PhantomData;

use crate::expression::*;
use crate::query_builder::*;
use crate::query_dsl::RunQueryDsl;
use crate::result::QueryResult;
use crate::sql_types::DieselNumericOps;

#[derive(Debug, Clone, DieselNumericOps)]
#[must_use = "Queries are only executed when calling `load`, `get_result`, or similar."]
/// Returned by the [`sql()`] function.
///
/// [`sql()`]: crate::dsl::sql()
pub struct SqlLiteral<ST, T = self::private::Empty> {
    sql: String,
    inner: T,
    _marker: PhantomData<ST>,
}

impl<ST, T> SqlLiteral<ST, T>
where
    ST: TypedExpressionType,
{
    pub(crate) fn new(sql: String, inner: T) -> Self {
        SqlLiteral {
            sql,
            inner,
            _marker: PhantomData,
        }
    }

    /// Bind a value for use with this SQL query.
    ///
    /// # Safety
    ///
    /// This function should be used with care, as Diesel cannot validate that
    /// the value is of the right type nor can it validate that you have passed
    /// the correct number of parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    users {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::sql_types::{Integer, Text, Bool};
    /// #     let connection = &mut establish_connection();
    /// let seans_id = users
    ///     .select(id)
    ///     .filter(sql::<Bool>("name = ").bind::<Text, _>("Sean"))
    ///     .get_result(connection);
    /// assert_eq!(Ok(1), seans_id);
    ///
    /// let tess_id = sql::<Integer>("SELECT id FROM users WHERE name = ")
    ///     .bind::<Text, _>("Tess")
    ///     .get_result(connection);
    /// assert_eq!(Ok(2), tess_id);
    /// # }
    /// ```
    ///
    /// ### Multiple Bind Params
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    users {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::sql_types::{Integer, Text, Bool};
    /// #     let connection = &mut establish_connection();
    /// #     diesel::insert_into(users).values(name.eq("Ryan"))
    /// #           .execute(connection).unwrap();
    /// let query = users
    ///     .select(name)
    ///     .filter(
    ///         sql::<Bool>("id > ")
    ///         .bind::<Integer,_>(1)
    ///         .sql(" AND name <> ")
    ///         .bind::<Text, _>("Ryan")
    ///     )
    ///     .get_results(connection);
    /// let expected = vec!["Tess".to_string()];
    /// assert_eq!(Ok(expected), query);
    /// # }
    /// ```
    pub fn bind<BindST, U>(self, bind_value: U) -> UncheckedBind<Self, U::Expression>
    where
        BindST: SqlType + TypedExpressionType,
        U: AsExpression<BindST>,
    {
        UncheckedBind::new(self, bind_value.as_expression())
    }

    /// Use literal SQL in the query builder
    ///
    /// This function is intended for use when you need a small bit of raw SQL in
    /// your query. If you want to write the entire query using raw SQL, use
    /// [`sql_query`](crate::sql_query()) instead.
    ///
    /// # Safety
    ///
    /// This function should be used with care, as Diesel cannot validate that
    /// the value is of the right type nor can it validate that you have passed
    /// the correct number of parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    users {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::sql_types::Bool;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::insert_into(users).values(name.eq("Ryan"))
    /// #           .execute(connection).unwrap();
    /// let query = users
    ///     .select(name)
    ///     .filter(
    ///         sql::<Bool>("id > 1")
    ///         .sql(" AND name <> 'Ryan'")
    ///     )
    ///     .get_results(connection);
    /// let expected = vec!["Tess".to_string()];
    /// assert_eq!(Ok(expected), query);
    /// # }
    /// ```
    pub fn sql(self, sql: &str) -> SqlLiteral<ST, Self> {
        SqlLiteral::new(sql.into(), self)
    }
}

impl<ST, T> Expression for SqlLiteral<ST, T>
where
    ST: TypedExpressionType,
{
    type SqlType = ST;
}

impl<ST, T, DB> QueryFragment<DB> for SqlLiteral<ST, T>
where
    DB: Backend,
    T: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.inner.walk_ast(out.reborrow())?;
        out.push_sql(&self.sql);
        Ok(())
    }
}

impl<ST, T> QueryId for SqlLiteral<ST, T> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<ST, T> Query for SqlLiteral<ST, T>
where
    Self: Expression,
{
    type SqlType = ST;
}

impl<ST, T, Conn> RunQueryDsl<Conn> for SqlLiteral<ST, T> {}

impl<QS, ST, T> SelectableExpression<QS> for SqlLiteral<ST, T> where Self: Expression {}

impl<QS, ST, T> AppearsOnTable<QS> for SqlLiteral<ST, T> where Self: Expression {}

impl<ST, T, GB> ValidGrouping<GB> for SqlLiteral<ST, T> {
    type IsAggregate = is_aggregate::Never;
}

/// Use literal SQL in the query builder.
///
/// Available for when you truly cannot represent something using the expression
/// DSL. You will need to provide the SQL type of the expression, in addition to
/// the SQL.
///
/// This function is intended for use when you need a small bit of raw SQL in
/// your query. If you want to write the entire query using raw SQL, use
/// [`sql_query`](crate::sql_query()) instead.
///
/// Query parameters can be bound into the literal SQL using [`SqlLiteral::bind()`].
///
/// # Safety
///
/// The compiler will be unable to verify the correctness of the annotated type.
/// If you give the wrong type, it'll either return an error when deserializing
/// the query result or produce unexpected values.
///
/// # Examples
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # fn main() {
/// #     run_test_1().unwrap();
/// #     run_test_2().unwrap();
/// # }
/// #
/// # fn run_test_1() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     use diesel::sql_types::Bool;
/// use diesel::dsl::sql;
/// #     let connection = &mut establish_connection();
/// let user = users.filter(sql::<Bool>("name = 'Sean'")).first(connection)?;
/// let expected = (1, String::from("Sean"));
/// assert_eq!(expected, user);
/// #     Ok(())
/// # }
/// #
/// # fn run_test_2() -> QueryResult<()> {
/// #     use crate::schema::users::dsl::*;
/// #     use diesel::dsl::sql;
/// #     use diesel::sql_types::{Bool, Integer, Text};
/// #     let connection = &mut establish_connection();
/// #     diesel::insert_into(users)
/// #         .values(name.eq("Ryan"))
/// #         .execute(connection).unwrap();
/// let query = users
///     .select(name)
///     .filter(
///         sql::<Bool>("id > ")
///         .bind::<Integer,_>(1)
///         .sql(" AND name <> ")
///         .bind::<Text, _>("Ryan")
///     )
///     .get_results(connection);
/// let expected = vec!["Tess".to_string()];
/// assert_eq!(Ok(expected), query);
/// #     Ok(())
/// # }
/// ```
/// [`SqlLiteral::bind()`]: crate::expression::SqlLiteral::bind()
pub fn sql<ST>(sql: &str) -> SqlLiteral<ST>
where
    ST: TypedExpressionType,
{
    SqlLiteral::new(sql.into(), self::private::Empty)
}

#[derive(QueryId, Debug, Clone, Copy)]
#[must_use = "Queries are only executed when calling `load`, `get_result`, or similar."]
/// Returned by the [`SqlLiteral::bind()`] method when binding a value to a fragment of SQL.
///
pub struct UncheckedBind<Query, Value> {
    query: Query,
    value: Value,
}

impl<Query, Value> UncheckedBind<Query, Value>
where
    Query: Expression,
{
    pub(crate) fn new(query: Query, value: Value) -> Self {
        UncheckedBind { query, value }
    }

    /// Use literal SQL in the query builder.
    ///
    /// This function is intended for use when you need a small bit of raw SQL in
    /// your query. If you want to write the entire query using raw SQL, use
    /// [`sql_query`](crate::sql_query()) instead.
    ///
    /// # Safety
    ///
    /// This function should be used with care, as Diesel cannot validate that
    /// the value is of the right type nor can it validate that you have passed
    /// the correct number of parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    users {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::sql_types::{Integer, Bool};
    /// #     let connection = &mut establish_connection();
    /// #     diesel::insert_into(users).values(name.eq("Ryan"))
    /// #           .execute(connection).unwrap();
    /// let query = users
    ///     .select(name)
    ///     .filter(
    ///         sql::<Bool>("id > ")
    ///         .bind::<Integer,_>(1)
    ///         .sql(" AND name <> 'Ryan'")
    ///     )
    ///     .get_results(connection);
    /// let expected = vec!["Tess".to_string()];
    /// assert_eq!(Ok(expected), query);
    /// # }
    /// ```
    pub fn sql(self, sql: &str) -> SqlLiteral<Query::SqlType, Self> {
        SqlLiteral::new(sql.into(), self)
    }
}

impl<Query, Value> Expression for UncheckedBind<Query, Value>
where
    Query: Expression,
{
    type SqlType = Query::SqlType;
}

impl<Query, Value, DB> QueryFragment<DB> for UncheckedBind<Query, Value>
where
    DB: Backend,
    Query: QueryFragment<DB>,
    Value: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.query.walk_ast(out.reborrow())?;
        self.value.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<Q, Value> Query for UncheckedBind<Q, Value>
where
    Q: Query,
{
    type SqlType = Q::SqlType;
}

impl<Query, Value, GB> ValidGrouping<GB> for UncheckedBind<Query, Value> {
    type IsAggregate = is_aggregate::Never;
}

impl<QS, Query, Value> SelectableExpression<QS> for UncheckedBind<Query, Value> where
    Self: AppearsOnTable<QS>
{
}

impl<QS, Query, Value> AppearsOnTable<QS> for UncheckedBind<Query, Value> where Self: Expression {}

impl<Query, Value, Conn> RunQueryDsl<Conn> for UncheckedBind<Query, Value> {}

mod private {
    use crate::backend::{Backend, DieselReserveSpecialization};
    use crate::query_builder::{QueryFragment, QueryId};

    #[derive(Debug, Clone, Copy, QueryId)]
    pub struct Empty;

    impl<DB> QueryFragment<DB> for Empty
    where
        DB: Backend + DieselReserveSpecialization,
    {
        fn walk_ast<'b>(
            &'b self,
            _pass: crate::query_builder::AstPass<'_, 'b, DB>,
        ) -> crate::QueryResult<()> {
            Ok(())
        }
    }
}
