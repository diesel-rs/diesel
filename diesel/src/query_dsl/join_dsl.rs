use query_builder::AsQuery;
use query_source::{JoinTo, QuerySource, Table};
use query_source::joins::OnClauseWrapper;

#[doc(hidden)]
/// `JoinDsl` support trait to emulate associated type constructors
pub trait InternalJoinDsl<Rhs, Kind, On> {
    type Output: AsQuery;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output;
}

impl<T, Rhs, Kind, On> InternalJoinDsl<Rhs, Kind, On> for T
where
    T: Table + AsQuery,
    T::Query: InternalJoinDsl<Rhs, Kind, On>,
{
    type Output = <T::Query as InternalJoinDsl<Rhs, Kind, On>>::Output;

    fn join(self, rhs: Rhs, kind: Kind, on: On) -> Self::Output {
        self.as_query().join(rhs, kind, on)
    }
}

#[doc(hidden)]
/// `JoinDsl` support trait to emulate associated type constructors and grab
/// the known on clause from the associations API
pub trait JoinWithImplicitOnClause<Rhs, Kind> {
    type Output: AsQuery;

    fn join_with_implicit_on_clause(self, rhs: Rhs, kind: Kind) -> Self::Output;
}

impl<Lhs, Rhs, Kind> JoinWithImplicitOnClause<Rhs, Kind> for Lhs
where
    Lhs: JoinTo<Rhs>,
    Lhs: InternalJoinDsl<<Lhs as JoinTo<Rhs>>::FromClause, Kind, <Lhs as JoinTo<Rhs>>::OnClause>,
{
    type Output = <Lhs as InternalJoinDsl<Lhs::FromClause, Kind, Lhs::OnClause>>::Output;

    fn join_with_implicit_on_clause(self, rhs: Rhs, kind: Kind) -> Self::Output {
        let (from, on) = Lhs::join_target(rhs);
        self.join(from, kind, on)
    }
}

/// Methods allowing various joins between two or more tables.
///
/// If you have invoked [`joinable!`] for the two tables, you can pass that
/// table directly.  Otherwise you will need to use [`.on`] to specify the `ON`
/// clause.
///
/// [`joinable!`]: ../macro.joinable.html
/// [`.on`]: trait.JoinOnDsl.html#method.on
///
/// You can join to as many tables as you'd like in a query, with the
/// restriction that no table can appear in the query more than once. The reason
/// for this restriction is that one of the appearances would require aliasing,
/// and we do not currently have a fleshed out story for dealing with table
/// aliases.
///
/// You may also need to call [`allow_tables_to_appear_in_same_query!`][] (particularly if
/// you see an unexpected error about `AppearsInFromClause`). See the
/// documentation for [`allow_tables_to_appear_in_same_query!`][] for details.
///
/// Diesel expects multi-table joins to be semantically grouped based on the
/// relationships. For example, `users.inner_join(posts.inner_join(comments))`
/// is not the same as `users.inner_join(posts).inner_join(comments)`. The first
/// would deserialize into `(User, (Post, Comment))` and generate the following
/// SQL:
///
/// ```sql
/// SELECT * FROM users
///     INNER JOIN posts ON posts.user_id = users.id
///     INNER JOIN comments ON comments.post_id = posts.id
/// ```
///
/// While the second query would deserialize into `(User, Post, Comment)` and
/// generate the following SQL:
///
/// ```sql
/// SELECT * FROM users
///     INNER JOIN posts ON posts.user_id = users.id
///     INNER JOIN comments ON comments.user_id = users.id
/// ```
///
/// [associations]: ../associations/index.html
/// [`allow_tables_to_appear_in_same_query!`]: ../macro.allow_tables_to_appear_in_same_query.html
pub trait JoinDsl: Sized {
    /// Join two tables using a SQL `INNER JOIN`.
    /// The implicit `ON` clause is defined
    /// via the [JoinDsl](./trait.JoinOnDsl.html).
    /// Use [`.on`](./trait.JoinOnDsl.html#method.on)
    /// to override the implicit `ON` clause.
    fn inner_join<Rhs>(self, rhs: Rhs) -> Self::Output
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::Inner>,
    {
        self.join_with_implicit_on_clause(rhs, joins::Inner)
    }

    /// Join two tables using a SQL `LEFT OUTER JOIN`.
    /// The implicit `ON` clause is defined
    /// via the [JoinDsl](./trait.JoinOnDsl.html).
    /// Use [`.on`](./trait.JoinOnDsl.html#method.on)
    /// to override the implicit `ON` clause.
    fn left_outer_join<Rhs>(self, rhs: Rhs) -> Self::Output
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::LeftOuter>,
    {
        self.join_with_implicit_on_clause(rhs, joins::LeftOuter)
    }

    /// Alias for `left_outer_join`
    fn left_join<Rhs>(self, rhs: Rhs) -> Self::Output
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::LeftOuter>,
    {
        self.left_outer_join(rhs)
    }
}

impl<T: AsQuery> JoinDsl for T {}

pub trait JoinOnDsl: Sized {
    /// Specify the `ON` clause for a join statement. This will override
    /// any implicit `ON` clause that would come from [`joinable!`]
    ///
    /// [`joinable!`]: ../macro.joinable.html
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, posts};
    /// #
    /// # fn main() {
    /// #     let connection = establish_connection();
    /// let data = users::table
    ///     .left_join(posts::table.on(
    ///         users::id.eq(posts::user_id).and(
    ///             posts::title.eq("My first post"))
    ///     ))
    ///     .select((users::name, posts::title.nullable()))
    ///     .load(&connection);
    /// let expected = vec![
    ///     ("Sean".to_string(), Some("My first post".to_string())),
    ///     ("Tess".to_string(), None),
    /// ];
    /// assert_eq!(Ok(expected), data);
    /// # }
    fn on<On>(self, on: On) -> OnClauseWrapper<Self, On> {
        OnClauseWrapper::new(self, on)
    }
}

impl<T: QuerySource> JoinOnDsl for T {}
