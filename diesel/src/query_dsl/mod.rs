//! Traits that construct SELECT statements
//!
//! Traits in this module have methods that generally map to the keyword for the corresponding clause in SQL,
//! unless it conflicts with a Rust keyword (such as `WHERE`/`where`).
//!
//! Methods for constructing queries lives on the [`QueryDsl`] trait.
//! Methods for executing queries live on [`RunQueryDsl`].
//!
//! See also [`expression_methods`][expression_methods] and [`dsl`][dsl].
//!
//! [expression_methods]: super::expression_methods
//! [dsl]: super::dsl

use crate::backend::Backend;
use crate::connection::Connection;
use crate::expression::count::CountStar;
use crate::expression::Expression;
use crate::helper_types::*;
use crate::query_builder::locking_clause as lock;
use crate::query_source::{joins, Table};
use crate::result::QueryResult;

mod belonging_to_dsl;
#[doc(hidden)]
pub mod boxed_dsl;
mod combine_dsl;
mod distinct_dsl;
#[doc(hidden)]
pub mod filter_dsl;
mod group_by_dsl;
mod having_dsl;
mod join_dsl;
#[doc(hidden)]
pub mod limit_dsl;
#[doc(hidden)]
pub mod load_dsl;
mod locking_dsl;
mod nullable_select_dsl;
mod offset_dsl;
pub(crate) mod order_dsl;
#[doc(hidden)]
pub mod positional_order_dsl;
mod save_changes_dsl;
#[doc(hidden)]
pub mod select_dsl;
mod single_value_dsl;

pub use self::belonging_to_dsl::BelongingToDsl;
pub use self::combine_dsl::CombineDsl;
pub use self::join_dsl::{InternalJoinDsl, JoinOnDsl, JoinWithImplicitOnClause};
#[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
pub use self::load_dsl::CompatibleType;
#[doc(hidden)]
pub use self::load_dsl::LoadQuery;
pub use self::save_changes_dsl::{SaveChangesDsl, UpdateAndFetchResults};

/// The traits used by `QueryDsl`.
///
/// Each trait in this module represents exactly one method from `QueryDsl`.
/// Apps should general rely on `QueryDsl` directly, rather than these traits.
/// However, generic code may need to include a where clause that references
/// these traits.
pub mod methods {
    pub use super::boxed_dsl::BoxedDsl;
    pub use super::distinct_dsl::*;
    #[doc(inline)]
    pub use super::filter_dsl::*;
    pub use super::group_by_dsl::GroupByDsl;
    pub use super::having_dsl::HavingDsl;
    pub use super::limit_dsl::LimitDsl;
    pub use super::load_dsl::{ExecuteDsl, LoadQuery};
    pub use super::locking_dsl::{LockingDsl, ModifyLockDsl};
    pub use super::nullable_select_dsl::SelectNullableDsl;
    pub use super::offset_dsl::OffsetDsl;
    pub use super::order_dsl::{OrderDsl, ThenOrderDsl};
    pub use super::select_dsl::SelectDsl;
    pub use super::single_value_dsl::SingleValueDsl;

    #[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
    #[doc(hidden)]
    #[allow(deprecated)]
    #[deprecated(note = "Use `LoadQuery::RowIter` directly")]
    pub use super::load_dsl::LoadRet;
}

/// Methods used to construct select statements.
pub trait QueryDsl: Sized {
    /// Adds the `DISTINCT` keyword to a query.
    ///
    /// This method will override any previous distinct clause that was present.
    /// For example, on PostgreSQL, `foo.distinct_on(bar).distinct()` will
    /// create the same query as `foo.distinct()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM users").execute(connection).unwrap();
    /// diesel::insert_into(users)
    ///     .values(&vec![name.eq("Sean"); 3])
    ///     .execute(connection)?;
    /// let names = users.select(name).load::<String>(connection)?;
    /// let distinct_names = users.select(name).distinct().load::<String>(connection)?;
    ///
    /// assert_eq!(vec!["Sean"; 3], names);
    /// assert_eq!(vec!["Sean"; 1], distinct_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn distinct(self) -> Distinct<Self>
    where
        Self: methods::DistinctDsl,
    {
        methods::DistinctDsl::distinct(self)
    }

    /// Adds the `DISTINCT ON` clause to a query.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::animals;
    /// #
    /// # #[derive(Queryable, Debug, PartialEq)]
    /// # struct Animal {
    /// #     species: String,
    /// #     name: Option<String>,
    /// #     legs: i32,
    /// # }
    /// #
    /// # impl Animal {
    /// #     fn new<S: Into<String>>(species: S, name: Option<&str>, legs: i32) -> Self {
    /// #         Animal {
    /// #             species: species.into(),
    /// #             name: name.map(Into::into),
    /// #             legs
    /// #         }
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM animals").execute(connection).unwrap();
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("dog"), name.eq(Some("Jack")), legs.eq(4)),
    ///         (species.eq("dog"), name.eq(None), legs.eq(4)),
    ///         (species.eq("spider"), name.eq(None), legs.eq(8)),
    ///     ])
    ///     .execute(connection)
    ///     .unwrap();
    /// let all_animals = animals.select((species, name, legs)).load(connection);
    /// let distinct_animals = animals
    ///     .select((species, name, legs))
    ///     .order_by((species, legs))
    ///     .distinct_on(species)
    ///     .load(connection);
    ///
    /// assert_eq!(Ok(vec![Animal::new("dog", Some("Jack"), 4),
    ///                    Animal::new("dog", None, 4),
    ///                    Animal::new("spider", None, 8)]), all_animals);
    /// assert_eq!(Ok(vec![Animal::new("dog", Some("Jack"), 4),
    ///                    Animal::new("spider", None, 8)]), distinct_animals);
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn distinct_on<Expr>(self, expr: Expr) -> DistinctOn<Self, Expr>
    where
        Self: methods::DistinctOnDsl<Expr>,
    {
        methods::DistinctOnDsl::distinct_on(self, expr)
    }

    // FIXME: Needs usage example and doc rewrite
    /// Adds a `SELECT` clause to the query.
    ///
    /// If there was already a select clause present, it will be overridden.
    /// For example, `foo.select(bar).select(baz)` will produce the same
    /// query as `foo.select(baz)`.
    ///
    /// By default, the select clause will be roughly equivalent to `SELECT *`
    /// (however, Diesel will list all columns to ensure that they are in the
    /// order we expect).
    ///
    /// `select` has slightly stricter bounds on its arguments than other
    /// methods. In particular, when used with a left outer join, `.nullable`
    /// must be called on columns that come from the right side of a join. It
    /// can be called on the column itself, or on an expression containing that
    /// column. `title.nullable()`, `lower(title).nullable()`, and `(id,
    /// title).nullable()` would all be valid.
    ///
    /// In order to use this method with columns from different tables
    /// a method like [`.inner_join`] or [`.left_join`] needs to be called before
    /// calling [`.select`] (See examples below).
    /// This is because you can only access columns from tables
    /// that appear in your query before that function call.
    ///
    /// [`.inner_join`]: QueryDsl::inner_join()
    /// [`.left_join`]: QueryDsl::left_join()
    /// [`.select`]: QueryDsl::select()
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// // By default, all columns will be selected
    /// let all_users = users.load::<(i32, String)>(connection)?;
    /// assert_eq!(vec![(1, String::from("Sean")), (2, String::from("Tess"))], all_users);
    ///
    /// let all_names = users.select(name).load::<String>(connection)?;
    /// assert_eq!(vec!["Sean", "Tess"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// ### When used with a left join
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, posts};
    /// #
    /// # #[derive(Queryable, PartialEq, Eq, Debug)]
    /// # struct User {
    /// #     id: i32,
    /// #     name: String,
    /// # }
    /// #
    /// # impl User {
    /// #     fn new(id: i32, name: &str) -> Self {
    /// #         User {
    /// #             id,
    /// #             name: name.into(),
    /// #         }
    /// #     }
    /// # }
    /// #
    /// # #[derive(Queryable, PartialEq, Eq, Debug)]
    /// # struct Post {
    /// #     id: i32,
    /// #     user_id: i32,
    /// #     title: String,
    /// # }
    /// #
    /// # impl Post {
    /// #     fn new(id: i32, user_id: i32, title: &str) -> Self {
    /// #         Post {
    /// #             id,
    /// #             user_id,
    /// #             title: title.into(),
    /// #         }
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let connection = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM posts").execute(connection)?;
    /// #     diesel::insert_into(posts::table)
    /// #         .values((posts::user_id.eq(1), posts::title.eq("Sean's Post")))
    /// #         .execute(connection)?;
    /// #     let post_id = posts::table.select(posts::id)
    /// #         .first::<i32>(connection)?;
    /// let join = users::table.left_join(posts::table);
    ///
    /// // By default, all columns from both tables are selected.
    /// // If no explicit select clause is used this means that the result
    /// // type of this query must contain all fields from the original schema in order.
    /// let all_data = join.load::<(User, Option<Post>)>(connection)?;
    /// let expected_data = vec![
    ///     (User::new(1, "Sean"), Some(Post::new(post_id, 1, "Sean's Post"))),
    ///     (User::new(2, "Tess"), None),
    /// ];
    /// assert_eq!(expected_data, all_data);
    ///
    /// // Since `posts` is on the right side of a left join, `.nullable` is
    /// // needed.
    /// let names_and_titles = join.select((users::name, posts::title.nullable()))
    ///     .load::<(String, Option<String>)>(connection)?;
    /// let expected_data = vec![
    ///     (String::from("Sean"), Some(String::from("Sean's Post"))),
    ///     (String::from("Tess"), None),
    /// ];
    /// assert_eq!(expected_data, names_and_titles);
    /// #     Ok(())
    /// # }
    /// ```
    fn select<Selection>(self, selection: Selection) -> Select<Self, Selection>
    where
        Selection: Expression,
        Self: methods::SelectDsl<Selection>,
    {
        methods::SelectDsl::select(self, selection)
    }

    /// Get the count of a query. This is equivalent to `.select(count_star())`
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let count = users.count().get_result(connection);
    /// assert_eq!(Ok(2), count);
    /// # }
    /// ```
    fn count(self) -> Select<Self, CountStar>
    where
        Self: methods::SelectDsl<CountStar>,
    {
        use crate::dsl::count_star;

        QueryDsl::select(self, count_star())
    }

    /// Join two tables using a SQL `INNER JOIN`.
    ///
    /// If you have invoked [`joinable!`] for the two tables, you can pass that
    /// table directly.  Otherwise you will need to use [`.on`] to specify the `ON`
    /// clause.
    ///
    /// [`joinable!`]: crate::joinable!
    /// [`.on`]: JoinOnDsl::on()
    ///
    /// You can join to as many tables as you'd like in a query, with the
    /// restriction that no table can appear in the query more than once. For
    /// tables that appear more than once in a single query the usage of [`alias!`](crate::alias!)
    /// is required.
    ///
    /// You will also need to call [`allow_tables_to_appear_in_same_query!`].
    /// If you are using `diesel print-schema`, this will
    /// have been generated for you.
    /// See the documentation for [`allow_tables_to_appear_in_same_query!`] for
    /// details.
    ///
    /// Diesel expects multi-table joins to be semantically grouped based on the
    /// relationships. For example, `users.inner_join(posts.inner_join(comments))`
    /// is not the same as `users.inner_join(posts).inner_join(comments)`. The first
    /// would deserialize into `(User, (Post, Comment))` and generate the following
    /// SQL:
    ///
    /// ```sql
    /// SELECT * FROM users
    ///     INNER JOIN (
    ///         posts
    ///         INNER JOIN comments ON comments.post_id = posts.id
    ///     ) ON posts.user_id = users.id
    ///
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
    /// The exact generated SQL may change in future diesel version as long as the
    /// generated query continues to produce same results. The currently generated
    /// SQL is referred as ["explicit join"](https://www.postgresql.org/docs/current/explicit-joins.html)
    /// by the PostgreSQL documentation and may have implications on the chosen query plan
    /// for large numbers of joins in the same query. Checkout the documentation of the
    /// [`join_collapse_limit` parameter](https://www.postgresql.org/docs/current/runtime-config-query.html#GUC-JOIN-COLLAPSE-LIMIT)
    /// to control this behaviour.
    ///
    /// [associations]: crate::associations
    /// [`allow_tables_to_appear_in_same_query!`]: crate::allow_tables_to_appear_in_same_query!
    ///
    /// Note that in order to use this method with [`.select`], you will need to use it before calling
    /// [`.select`] (See examples below). This is because you can only access columns from tables
    /// that appear in your query before the call to [`.select`].
    ///
    /// [`.select`]: QueryDsl::select()
    ///
    /// # Examples
    ///
    /// ### With implicit `ON` clause
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, posts};
    /// # /*
    /// joinable!(posts -> users (user_id));
    /// allow_tables_to_appear_in_same_query!(users, posts);
    /// # */
    ///
    /// # fn main() {
    /// #     use self::users::dsl::{users, name};
    /// #     use self::posts::dsl::{posts, user_id, title};
    /// #     let connection = &mut establish_connection();
    /// let data = users.inner_join(posts)
    ///     .select((name, title))
    ///     .load(connection);
    ///
    /// let expected_data = vec![
    ///     (String::from("Sean"), String::from("My first post")),
    ///     (String::from("Sean"), String::from("About Rust")),
    ///     (String::from("Tess"), String::from("My first post too")),
    /// ];
    /// assert_eq!(Ok(expected_data), data);
    /// # }
    /// ```
    ///
    /// ### With explicit `ON` clause
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, posts};
    /// #
    /// # /*
    /// allow_tables_to_appear_in_same_query!(users, posts);
    /// # */
    ///
    /// # fn main() {
    /// #     use self::users::dsl::{users, name};
    /// #     use self::posts::dsl::{posts, user_id, title};
    /// #     let connection = &mut establish_connection();
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         (user_id.eq(1), title.eq("Sean's post")),
    ///         (user_id.eq(2), title.eq("Sean is a jerk")),
    ///     ])
    ///     .execute(connection)
    ///     .unwrap();
    ///
    /// let data = users
    ///     .inner_join(posts.on(title.like(name.concat("%"))))
    ///     .select((name, title))
    ///     .load(connection);
    /// let expected_data = vec![
    ///     (String::from("Sean"), String::from("Sean's post")),
    ///     (String::from("Sean"), String::from("Sean is a jerk")),
    /// ];
    /// assert_eq!(Ok(expected_data), data);
    /// # }
    /// ```
    ///
    /// ### With explicit `ON` clause (struct)
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, posts};
    /// #
    /// # /*
    /// allow_tables_to_appear_in_same_query!(users, posts);
    /// # */
    ///
    /// # fn main() {
    /// #     use self::users::dsl::{users, name};
    /// #     use self::posts::dsl::{posts, user_id, title};
    /// #     let connection = &mut establish_connection();
    /// #[derive(Debug, PartialEq, Queryable)]
    /// struct User {
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// #[derive(Debug, PartialEq, Queryable)]
    /// struct Post {
    ///     id: i32,
    ///     user_id: i32,
    ///     title: String,
    /// }
    ///
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         (user_id.eq(1), title.eq("Sean's post")),
    ///         (user_id.eq(2), title.eq("Sean is a jerk")),
    ///     ])
    ///     .execute(connection)
    ///     .unwrap();
    ///
    /// // By default, all columns from both tables are selected.
    /// // If no explicit select clause is used this means that the
    /// // result type of this query must contain all fields from the
    /// // original schema in order.
    /// let data = users
    ///     .inner_join(posts.on(title.like(name.concat("%"))))
    ///     .load::<(User, Post)>(connection); // type could be elided
    /// let expected_data = vec![
    ///     (
    ///         User { id: 1, name: String::from("Sean") },
    ///         Post { id: 4, user_id: 1, title: String::from("Sean's post") },
    ///     ),
    ///     (
    ///         User { id: 1, name: String::from("Sean") },
    ///         Post { id: 5, user_id: 2, title: String::from("Sean is a jerk") },
    ///     ),
    /// ];
    /// assert_eq!(Ok(expected_data), data);
    /// # }
    /// ```
    fn inner_join<Rhs>(self, rhs: Rhs) -> InnerJoin<Self, Rhs>
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::Inner>,
    {
        self.join_with_implicit_on_clause(rhs, joins::Inner)
    }

    /// Join two tables using a SQL `LEFT OUTER JOIN`.
    ///
    /// Behaves similarly to [`inner_join`], but will produce a left join
    /// instead. See [`inner_join`] for usage examples.
    ///
    /// [`inner_join`]: QueryDsl::inner_join()
    ///
    /// Columns in the right hand table will become `Nullable` which means
    /// you must call `nullable()` on the corresponding fields in the select
    /// clause:
    ///
    /// ### Selecting after a left join
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, posts};
    /// #
    /// # #[derive(Queryable, PartialEq, Eq, Debug)]
    /// # struct User {
    /// #     id: i32,
    /// #     name: String,
    /// # }
    /// #
    /// # impl User {
    /// #     fn new(id: i32, name: &str) -> Self {
    /// #         User {
    /// #             id,
    /// #             name: name.into(),
    /// #         }
    /// #     }
    /// # }
    /// #
    /// # #[derive(Queryable, PartialEq, Eq, Debug)]
    /// # struct Post {
    /// #     id: i32,
    /// #     user_id: i32,
    /// #     title: String,
    /// # }
    /// #
    /// # impl Post {
    /// #     fn new(id: i32, user_id: i32, title: &str) -> Self {
    /// #         Post {
    /// #             id,
    /// #             user_id,
    /// #             title: title.into(),
    /// #         }
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let connection = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM posts").execute(connection)?;
    /// #     diesel::insert_into(posts::table)
    /// #         .values((posts::user_id.eq(1), posts::title.eq("Sean's Post")))
    /// #         .execute(connection)?;
    /// #     let post_id = posts::table.select(posts::id)
    /// #         .first::<i32>(connection)?;
    /// let join = users::table.left_join(posts::table);
    ///
    /// // Since `posts` is on the right side of a left join, `.nullable` is
    /// // needed.
    /// let names_and_titles = join.select((users::name, posts::title.nullable()))
    ///     .load::<(String, Option<String>)>(connection)?;
    /// let expected_data = vec![
    ///     (String::from("Sean"), Some(String::from("Sean's Post"))),
    ///     (String::from("Tess"), None),
    /// ];
    /// assert_eq!(expected_data, names_and_titles);
    /// #     Ok(())
    /// # }
    /// ```
    fn left_outer_join<Rhs>(self, rhs: Rhs) -> LeftJoin<Self, Rhs>
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::LeftOuter>,
    {
        self.join_with_implicit_on_clause(rhs, joins::LeftOuter)
    }

    /// Alias for [`left_outer_join`].
    ///
    /// [`left_outer_join`]: QueryDsl::left_outer_join()
    fn left_join<Rhs>(self, rhs: Rhs) -> LeftJoin<Self, Rhs>
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::LeftOuter>,
    {
        self.left_outer_join(rhs)
    }

    /// Adds to the `WHERE` clause of a query.
    ///
    /// If there is already a `WHERE` clause, the result will be `old AND new`.
    ///
    /// Note that in order to use this method with columns from different tables, you need to call
    ///  [`.inner_join`] or [`.left_join`] beforehand.
    /// This is because you can only access columns from tables
    /// that appear in your query before the call to [`.filter`].
    ///
    /// [`.inner_join`]: QueryDsl::inner_join()
    /// [`.left_join`]: QueryDsl::left_join()
    /// [`.filter`]: QueryDsl::filter()
    ///
    /// # Example:
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let seans_id = users.filter(name.eq("Sean")).select(id)
    ///     .first(connection);
    /// assert_eq!(Ok(1), seans_id);
    /// let tess_id = users.filter(name.eq("Tess")).select(id)
    ///     .first(connection);
    /// assert_eq!(Ok(2), tess_id);
    /// # }
    /// ```
    #[doc(alias = "where")]
    fn filter<Predicate>(self, predicate: Predicate) -> Filter<Self, Predicate>
    where
        Self: methods::FilterDsl<Predicate>,
    {
        methods::FilterDsl::filter(self, predicate)
    }

    /// Adds to the `WHERE` clause of a query using `OR`
    ///
    /// If there is already a `WHERE` clause, the result will be `(old OR new)`.
    /// Calling `foo.filter(bar).or_filter(baz)`
    /// is identical to `foo.filter(bar.or(baz))`.
    /// However, the second form is much harder to do dynamically.
    ///
    /// # Example:
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::delete(animals).execute(connection)?;
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("cat"), legs.eq(4), name.eq("Sinatra")),
    ///         (species.eq("dog"), legs.eq(3), name.eq("Fido")),
    ///         (species.eq("spider"), legs.eq(8), name.eq("Charlotte")),
    ///     ])
    ///     .execute(connection)?;
    ///
    /// let good_animals = animals
    ///     .filter(name.eq("Fido"))
    ///     .or_filter(legs.eq(4))
    ///     .select(name)
    ///     .get_results::<Option<String>>(connection)?;
    /// let expected = vec![
    ///     Some(String::from("Sinatra")),
    ///     Some(String::from("Fido")),
    /// ];
    /// assert_eq!(expected, good_animals);
    /// #     Ok(())
    /// # }
    /// ```
    #[doc(alias = "where")]
    fn or_filter<Predicate>(self, predicate: Predicate) -> OrFilter<Self, Predicate>
    where
        Self: methods::OrFilterDsl<Predicate>,
    {
        methods::OrFilterDsl::or_filter(self, predicate)
    }

    /// Attempts to find a single record from the given table by primary key.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     use diesel::result::Error::NotFound;
    /// #     let connection = &mut establish_connection();
    /// let sean = (1, "Sean".to_string());
    /// let tess = (2, "Tess".to_string());
    /// assert_eq!(Ok(sean), users.find(1).first(connection));
    /// assert_eq!(Ok(tess), users.find(2).first(connection));
    /// assert_eq!(Err::<(i32, String), _>(NotFound), users.find(3).first(connection));
    /// # }
    /// ```
    fn find<PK>(self, id: PK) -> Find<Self, PK>
    where
        Self: methods::FindDsl<PK>,
    {
        methods::FindDsl::find(self, id)
    }

    /// Sets the order clause of a query.
    ///
    /// If there was already an order clause, it will be overridden. See
    /// also:
    /// [`.desc()`](crate::expression_methods::ExpressionMethods::desc())
    /// and
    /// [`.asc()`](crate::expression_methods::ExpressionMethods::asc())
    ///
    /// Ordering by multiple columns can be achieved by passing a tuple of those
    /// columns.
    /// To construct an order clause of an unknown number of columns,
    /// see [`QueryDsl::then_order_by`](QueryDsl::then_order_by())
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM users").execute(connection)?;
    /// diesel::insert_into(users)
    ///     .values(&vec![
    ///         name.eq("Saul"),
    ///         name.eq("Steve"),
    ///         name.eq("Stan"),
    ///     ])
    ///     .execute(connection)?;
    ///
    /// let ordered_names = users.select(name)
    ///     .order(name.desc())
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Steve", "Stan", "Saul"], ordered_names);
    ///
    /// diesel::insert_into(users).values(name.eq("Stan")).execute(connection)?;
    ///
    /// let data = users.select((name, id))
    ///     .order((name.asc(), id.desc()))
    ///     .load(connection)?;
    /// let expected_data = vec![
    ///     (String::from("Saul"), 3),
    ///     (String::from("Stan"), 6),
    ///     (String::from("Stan"), 5),
    ///     (String::from("Steve"), 4),
    /// ];
    /// assert_eq!(expected_data, data);
    /// #    Ok(())
    /// # }
    /// ```
    fn order<Expr>(self, expr: Expr) -> Order<Self, Expr>
    where
        Expr: Expression,
        Self: methods::OrderDsl<Expr>,
    {
        methods::OrderDsl::order(self, expr)
    }

    /// Alias for `order`
    fn order_by<Expr>(self, expr: Expr) -> OrderBy<Self, Expr>
    where
        Expr: Expression,
        Self: methods::OrderDsl<Expr>,
    {
        QueryDsl::order(self, expr)
    }

    /// Appends to the `ORDER BY` clause of this SQL query.
    ///
    /// Unlike `.order`, this method will append rather than replace.
    /// In other words,
    /// `.order_by(foo).order_by(bar)` is equivalent to `.order_by(bar)`.
    /// In contrast,
    /// `.order_by(foo).then_order_by(bar)` is equivalent to `.order((foo, bar))`.
    /// This method is only present on boxed queries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM users").execute(connection)?;
    /// diesel::insert_into(users)
    ///     .values(&vec![
    ///         name.eq("Saul"),
    ///         name.eq("Steve"),
    ///         name.eq("Stan"),
    ///         name.eq("Stan"),
    ///     ])
    ///     .execute(connection)?;
    ///
    /// let data = users.select((name, id))
    ///     .order_by(name.asc())
    ///     .then_order_by(id.desc())
    ///     .load(connection)?;
    /// let expected_data = vec![
    ///     (String::from("Saul"), 3),
    ///     (String::from("Stan"), 6),
    ///     (String::from("Stan"), 5),
    ///     (String::from("Steve"), 4),
    /// ];
    /// assert_eq!(expected_data, data);
    /// #    Ok(())
    /// # }
    /// ```
    fn then_order_by<Order>(self, order: Order) -> ThenOrderBy<Self, Order>
    where
        Self: methods::ThenOrderDsl<Order>,
    {
        methods::ThenOrderDsl::then_order_by(self, order)
    }

    /// Sets the limit clause of the query.
    ///
    /// If there was already a limit clause, it will be overridden.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::delete(users).execute(connection)?;
    /// #     diesel::insert_into(users)
    /// #        .values(&vec![
    /// #            name.eq("Sean"),
    /// #            name.eq("Bastien"),
    /// #            name.eq("Pascal"),
    /// #        ])
    /// #        .execute(connection)?;
    /// #
    /// // Using a limit
    /// let limited = users.select(name)
    ///     .order(id)
    ///     .limit(1)
    ///     .load::<String>(connection)?;
    ///
    /// // Without a limit
    /// let no_limit = users.select(name)
    ///     .order(id)
    ///     .load::<String>(connection)?;
    ///
    /// assert_eq!(vec!["Sean"], limited);
    /// assert_eq!(vec!["Sean", "Bastien", "Pascal"], no_limit);
    /// #    Ok(())
    /// # }
    /// ```
    fn limit(self, limit: i64) -> Limit<Self>
    where
        Self: methods::LimitDsl,
    {
        methods::LimitDsl::limit(self, limit)
    }

    /// Sets the offset clause of the query.
    ///
    /// If there was already a offset clause, it will be overridden.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::delete(users).execute(connection)?;
    /// #     diesel::insert_into(users)
    /// #        .values(&vec![
    /// #            name.eq("Sean"),
    /// #            name.eq("Bastien"),
    /// #            name.eq("Pascal"),
    /// #        ])
    /// #        .execute(connection)?;
    /// #
    /// // Using an offset
    /// let offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .offset(1)
    ///     .load::<String>(connection)?;
    ///
    /// // No Offset
    /// let no_offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .load::<String>(connection)?;
    ///
    /// assert_eq!(vec!["Bastien", "Pascal"], offset);
    /// assert_eq!(vec!["Sean", "Bastien"], no_offset);
    /// #     Ok(())
    /// # }
    /// ```
    fn offset(self, offset: i64) -> Offset<Self>
    where
        Self: methods::OffsetDsl,
    {
        methods::OffsetDsl::offset(self, offset)
    }

    /// Sets the `group by` clause of a query.
    ///
    /// **Note:** Queries having a `group by` clause require a custom select clause.
    /// Use [`QueryDsl::select()`] to specify one.
    ///
    /// If there was already a group by clause, it will be overridden.
    /// Grouping by multiple columns can be achieved by passing a tuple of those
    /// columns.
    ///
    /// Diesel follows postgresql's group by semantic, this means any column
    /// appearing in a group by clause is considered to be aggregated. If a
    /// primary key is part of the group by clause every column from the
    /// corresponding table is considered to be aggregated. Select clauses
    /// cannot mix aggregated and non aggregated expressions.
    ///
    /// For group by clauses containing columns from more than one table it
    /// is required to call [`allow_columns_to_appear_in_same_group_by_clause!`]
    ///
    /// [`allow_columns_to_appear_in_same_group_by_clause!`]: crate::allow_columns_to_appear_in_same_group_by_clause!
    ///
    /// # Examples
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::{users, posts};
    /// #     use diesel::dsl::count;
    /// #     let connection = &mut establish_connection();
    /// let data = users::table.inner_join(posts::table)
    ///     .group_by(users::id)
    ///     .select((users::name, count(posts::id)))
    /// #   .order_by(users::id.asc())
    ///     .load::<(String, i64)>(connection)?;
    ///
    /// assert_eq!(vec![(String::from("Sean"), 2), (String::from("Tess"), 1)], data);
    /// # Ok(())
    /// # }
    /// ```
    fn group_by<GB>(self, group_by: GB) -> GroupBy<Self, GB>
    where
        GB: Expression,
        Self: methods::GroupByDsl<GB>,
    {
        methods::GroupByDsl::group_by(self, group_by)
    }

    /// Adds to the `HAVING` clause of a query.
    ///
    /// # Examples
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::{users, posts};
    /// #     use diesel::dsl::count;
    /// #     let connection = &mut establish_connection();
    /// let data = users::table.inner_join(posts::table)
    ///     .group_by(users::id)
    ///     .having(count(posts::id).gt(1))
    ///     .select((users::name, count(posts::id)))
    ///     .load::<(String, i64)>(connection)?;
    ///
    /// assert_eq!(vec![(String::from("Sean"), 2)], data);
    /// # Ok(())
    /// # }
    /// ```
    fn having<Predicate>(self, predicate: Predicate) -> Having<Self, Predicate>
    where
        Self: methods::HavingDsl<Predicate>,
    {
        methods::HavingDsl::having(self, predicate)
    }

    /// Adds `FOR UPDATE` to the end of the select statement.
    ///
    /// This method is only available for MySQL and PostgreSQL. SQLite does not
    /// provide any form of row locking.
    ///
    /// Additionally, `.for_update` cannot be used on queries with a distinct
    /// clause, group by clause, having clause, or any unions. Queries with
    /// a `FOR UPDATE` clause cannot be boxed.
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # #[cfg(any(feature = "mysql", feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::users;
    /// #     let connection = &mut establish_connection();
    /// // Executes `SELECT * FROM users FOR UPDATE`
    /// let users_for_update = users::table.for_update().load(connection)?;
    /// # let u: Vec<(i32, String)> = users_for_update;
    /// # Ok(())
    /// # }
    /// # #[cfg(feature = "sqlite")]
    /// # fn run_test() -> QueryResult<()> { Ok(()) }
    /// ```
    fn for_update(self) -> ForUpdate<Self>
    where
        Self: methods::LockingDsl<lock::ForUpdate>,
    {
        methods::LockingDsl::with_lock(self, lock::ForUpdate)
    }

    /// Adds `FOR NO KEY UPDATE` to the end of the select statement.
    ///
    /// This method is only available for PostgreSQL. SQLite does not
    /// provide any form of row locking, and MySQL does not support anything
    /// finer than row-level locking.
    ///
    /// Additionally, `.for_no_key_update` cannot be used on queries with a distinct
    /// clause, group by clause, having clause, or any unions. Queries with
    /// a `FOR NO KEY UPDATE` clause cannot be boxed.
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::users;
    /// #     let connection = &mut establish_connection();
    /// // Executes `SELECT * FROM users FOR NO KEY UPDATE`
    /// let users_for_no_key_update = users::table.for_no_key_update().load(connection)?;
    /// # let u: Vec<(i32, String)> = users_for_no_key_update;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> { Ok(()) }
    /// ```
    fn for_no_key_update(self) -> ForNoKeyUpdate<Self>
    where
        Self: methods::LockingDsl<lock::ForNoKeyUpdate>,
    {
        methods::LockingDsl::with_lock(self, lock::ForNoKeyUpdate)
    }

    /// Adds `FOR SHARE` to the end of the select statement.
    ///
    /// This method is only available for MySQL and PostgreSQL. SQLite does not
    /// provide any form of row locking.
    ///
    /// Additionally, `.for_share` cannot be used on queries with a distinct
    /// clause, group by clause, having clause, or any unions. Queries with
    /// a `FOR SHARE` clause cannot be boxed.
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # #[cfg(any(feature = "mysql", feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::users;
    /// #     let connection = &mut establish_connection();
    /// // Executes `SELECT * FROM users FOR SHARE`
    /// let users_for_share = users::table.for_share().load(connection)?;
    /// # let u: Vec<(i32, String)> = users_for_share;
    /// # Ok(())
    /// # }
    /// # #[cfg(feature = "sqlite")]
    /// # fn run_test() -> QueryResult<()> { Ok(()) }
    /// ```
    fn for_share(self) -> ForShare<Self>
    where
        Self: methods::LockingDsl<lock::ForShare>,
    {
        methods::LockingDsl::with_lock(self, lock::ForShare)
    }

    /// Adds `FOR KEY SHARE` to the end of the select statement.
    ///
    /// This method is only available for PostgreSQL. SQLite does not
    /// provide any form of row locking, and MySQL does not support anything
    /// finer than row-level locking.
    ///
    /// Additionally, `.for_key_share` cannot be used on queries with a distinct
    /// clause, group by clause, having clause, or any unions. Queries with
    /// a `FOR KEY SHARE` clause cannot be boxed.
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    ///
    /// # #[cfg(feature = "postgres")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::users;
    /// #     let connection = &mut establish_connection();
    /// // Executes `SELECT * FROM users FOR KEY SHARE`
    /// let users_for_key_share = users::table.for_key_share().load(connection)?;
    /// # let u: Vec<(i32, String)> = users_for_key_share;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> { Ok(()) }
    /// ```
    fn for_key_share(self) -> ForKeyShare<Self>
    where
        Self: methods::LockingDsl<lock::ForKeyShare>,
    {
        methods::LockingDsl::with_lock(self, lock::ForKeyShare)
    }

    /// Adds `SKIP LOCKED` to the end of a `FOR UPDATE` clause.
    ///
    /// This modifier is only supported in PostgreSQL 9.5+ and MySQL 8+.
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # #[cfg(any(feature = "postgres", feature = "mysql"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::users;
    /// #     let connection = &mut establish_connection();
    /// // Executes `SELECT * FROM users FOR UPDATE SKIP LOCKED`
    /// let user_skipped_locked = users::table.for_update().skip_locked().load(connection)?;
    /// # let u: Vec<(i32, String)> = user_skipped_locked;
    /// # Ok(())
    /// # }
    /// # #[cfg(feature = "sqlite")]
    /// # fn run_test() -> QueryResult<()> { Ok(()) }
    /// ```
    fn skip_locked(self) -> SkipLocked<Self>
    where
        Self: methods::ModifyLockDsl<lock::SkipLocked>,
    {
        methods::ModifyLockDsl::modify_lock(self, lock::SkipLocked)
    }

    /// Adds `NOWAIT` to the end of a `FOR UPDATE` clause.
    ///
    /// This modifier is only supported in PostgreSQL 9.5+ and MySQL 8+.
    ///
    /// # Example
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # #[cfg(any(feature = "mysql", feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use crate::schema::users;
    /// #     let connection = &mut establish_connection();
    /// // Executes `SELECT * FROM users FOR UPDATE NOWAIT`
    /// let users_no_wait = users::table.for_update().no_wait().load(connection)?;
    /// # let u: Vec<(i32, String)> = users_no_wait;
    /// # Ok(())
    /// # }
    /// # #[cfg(feature = "sqlite")]
    /// # fn run_test() -> QueryResult<()> { Ok(()) }
    /// ```
    fn no_wait(self) -> NoWait<Self>
    where
        Self: methods::ModifyLockDsl<lock::NoWait>,
    {
        methods::ModifyLockDsl::modify_lock(self, lock::NoWait)
    }

    /// Boxes the pieces of a query into a single type.
    ///
    /// This is useful for cases where you want to conditionally modify a query,
    /// but need the type to remain the same. The backend must be specified as
    /// part of this. It is not possible to box a query and have it be useable
    /// on multiple backends.
    ///
    /// A boxed query will incur a minor performance penalty, as the query builder
    /// can no longer be inlined by the compiler. For most applications this cost
    /// will be minimal.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     use std::collections::HashMap;
    /// #     let connection = &mut establish_connection();
    /// #     let mut params = HashMap::new();
    /// #     params.insert("name", "Sean");
    /// let mut query = users::table.into_boxed();
    /// if let Some(name) = params.get("name") {
    ///     query = query.filter(users::name.eq(name));
    /// }
    /// let users = query.load(connection);
    /// #     let expected = vec![(1, String::from("Sean"))];
    /// #     assert_eq!(Ok(expected), users);
    /// # }
    /// ```
    ///
    /// Diesel queries also have a similar problem to [`Iterator`], where
    /// returning them from a function requires exposing the implementation of that
    /// function. The [`helper_types`][helper_types] module exists to help with this,
    /// but you might want to hide the return type or have it conditionally change.
    /// Boxing can achieve both.
    ///
    /// [helper_types]: crate::helper_types
    ///
    /// ### Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     let connection = &mut establish_connection();
    /// fn users_by_name(name: &str) -> users::BoxedQuery<DB> {
    ///     users::table.filter(users::name.eq(name)).into_boxed()
    /// }
    ///
    /// assert_eq!(Ok(1), users_by_name("Sean").select(users::id).first(connection));
    /// assert_eq!(Ok(2), users_by_name("Tess").select(users::id).first(connection));
    /// # }
    /// ```
    fn into_boxed<'a, DB>(self) -> IntoBoxed<'a, Self, DB>
    where
        DB: Backend,
        Self: methods::BoxedDsl<'a, DB>,
    {
        methods::BoxedDsl::internal_into_boxed(self)
    }

    /// Wraps this select statement in parenthesis, allowing it to be used
    /// as an expression.
    ///
    /// SQL allows queries such as `foo = (SELECT ...)`, as long as the
    /// subselect returns only a single column, and 0 or 1 rows. This method
    /// indicates that you expect the query to only return a single value (this
    /// will be enforced by adding `LIMIT 1`).
    ///
    /// The SQL type of this will always be `Nullable`, as the query returns
    /// `NULL` if the table is empty or it otherwise returns 0 rows.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     use schema::posts;
    /// #     let connection = &mut establish_connection();
    /// insert_into(posts::table)
    ///     .values(posts::user_id.eq(1))
    ///     .execute(connection)?;
    /// let last_post = posts::table
    ///     .order(posts::id.desc());
    /// let most_recently_active_user = users.select(name)
    ///     .filter(id.nullable().eq(last_post.select(posts::user_id).single_value()))
    ///     .first::<String>(connection)?;
    /// assert_eq!("Sean", most_recently_active_user);
    /// #     Ok(())
    /// # }
    /// ```
    fn single_value(self) -> SingleValue<Self>
    where
        Self: methods::SingleValueDsl,
    {
        methods::SingleValueDsl::single_value(self)
    }

    /// Coerce the SQL type of the select clause to it's nullable equivalent.
    ///
    /// This is useful for writing queries that contain subselects on non null
    /// fields comparing them to nullable fields.
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #    run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let connection = &mut establish_connection();
    /// table! {
    ///     users {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// table! {
    ///     posts {
    ///         id -> Integer,
    ///         by_user -> Nullable<Text>,
    ///     }
    /// }
    ///
    /// allow_tables_to_appear_in_same_query!(users, posts);
    ///
    /// # let _: Vec<(i32, Option<String>)> =
    /// posts::table.filter(
    ///    posts::by_user.eq_any(users::table.select(users::name).nullable())
    /// ).load(connection)?;
    /// #     Ok(())
    /// # }
    fn nullable(self) -> NullableSelect<Self>
    where
        Self: methods::SelectNullableDsl,
    {
        methods::SelectNullableDsl::nullable(self)
    }
}

impl<T: Table> QueryDsl for T {}

/// Methods used to execute queries.
pub trait RunQueryDsl<Conn>: Sized {
    /// Executes the given command, returning the number of rows affected.
    ///
    /// `execute` is usually used in conjunction with [`insert_into`](crate::insert_into()),
    /// [`update`](crate::update()) and [`delete`](crate::delete()) where the number of
    /// affected rows is often enough information.
    ///
    /// When asking the database to return data from a query, [`load`](crate::query_dsl::RunQueryDsl::load()) should
    /// probably be used instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let inserted_rows = insert_into(users)
    ///     .values(name.eq("Ruby"))
    ///     .execute(connection)?;
    /// assert_eq!(1, inserted_rows);
    ///
    /// let inserted_rows = insert_into(users)
    ///     .values(&vec![name.eq("Jim"), name.eq("James")])
    ///     .execute(connection)?;
    /// assert_eq!(2, inserted_rows);
    /// #     Ok(())
    /// # }
    /// ```
    fn execute(self, conn: &mut Conn) -> QueryResult<usize>
    where
        Conn: Connection,
        Self: methods::ExecuteDsl<Conn>,
    {
        methods::ExecuteDsl::execute(self, conn)
    }

    /// Executes the given query, returning a [`Vec`] with the returned rows.
    ///
    /// When using the query builder, the return type can be
    /// a tuple of the values, or a struct which implements [`Queryable`].
    ///
    /// When this method is called on [`sql_query`],
    /// the return type can only be a struct which implements [`QueryableByName`]
    ///
    /// For insert, update, and delete operations where only a count of affected is needed,
    /// [`execute`] should be used instead.
    ///
    /// [`Queryable`]: crate::deserialize::Queryable
    /// [`QueryableByName`]: crate::deserialize::QueryableByName
    /// [`execute`]: crate::query_dsl::RunQueryDsl::execute()
    /// [`sql_query`]: crate::sql_query()
    ///
    /// ## How to resolve compiler errors while loading data from the database
    ///
    /// In case you getting uncomprehensable compiler errors while loading data
    /// from the database into a type using [`#[derive(Queryable)]`](derive@crate::prelude::Queryable)
    /// you might want to consider
    /// using  [`#[derive(Selectable)]`](derive@crate::prelude::Selectable) +
    /// `#[diesel(check_for_backend(YourBackendType))]`
    /// to check for mismatching fields at compile time. This drastically improves
    /// the quality of the generated error messages by pointing to concrete type mismatches at
    /// field level.You need to specify the concrete database backend
    /// this specific struct is indented to be used with, as otherwise rustc cannot correctly
    /// identify the required deserialization implementation.
    ///
    /// # Examples
    ///
    /// ## Returning a single field
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let data = users.select(name)
    ///     .load::<String>(connection)?;
    /// assert_eq!(vec!["Sean", "Tess"], data);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// ## Returning a tuple
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let data = users
    ///     .load::<(i32, String)>(connection)?;
    /// let expected_data = vec![
    ///     (1, String::from("Sean")),
    ///     (2, String::from("Tess")),
    /// ];
    /// assert_eq!(expected_data, data);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// ## Returning a struct
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// #[derive(Queryable, PartialEq, Debug)]
    /// struct User {
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let data = users
    ///     .load::<User>(connection)?;
    /// let expected_data = vec![
    ///     User { id: 1, name: String::from("Sean") },
    ///     User { id: 2, name: String::from("Tess") },
    /// ];
    /// assert_eq!(expected_data, data);
    /// #     Ok(())
    /// # }
    /// ```
    fn load<'query, U>(self, conn: &mut Conn) -> QueryResult<Vec<U>>
    where
        Self: LoadQuery<'query, Conn, U>,
    {
        self.internal_load(conn)?.collect()
    }

    /// Executes the given query, returning an [`Iterator`] with the returned rows.
    ///
    /// The iterator's item is [`QueryResult<U>`](crate::result::QueryResult).
    ///
    /// You should normally prefer to use [`RunQueryDsl::load`] instead. This method
    /// is provided for situations where the result needs to be collected into a different
    /// container than a [`Vec`]
    ///
    /// When using the query builder, the return type can be
    /// a tuple of the values, or a struct which implements [`Queryable`].
    /// This type is specified by the first generic type of this function.
    ///
    /// The second generic type parameter specifies the so called loading mode,
    /// which describes how the connection implementation loads data from the database.
    /// All connections should provide a implementation for
    /// [`DefaultLoadingMode`](crate::connection::DefaultLoadingMode).
    ///
    /// They may provide additional modes. Checkout the documentation of the concrete
    /// connection types for details. For connection implementations that provide
    /// more than one loading mode it is **required** to specify this generic parameter.
    /// This is currently true for `PgConnection`.
    ///
    /// When this method is called on [`sql_query`],
    /// the return type can only be a struct which implements [`QueryableByName`]
    ///
    /// For insert, update, and delete operations where only a count of affected is needed,
    /// [`execute`] should be used instead.
    ///
    /// [`Queryable`]: crate::deserialize::Queryable
    /// [`QueryableByName`]: crate::deserialize::QueryableByName
    /// [`execute`]: crate::query_dsl::RunQueryDsl::execute()
    /// [`sql_query`]: crate::sql_query()
    ///
    /// # Examples
    ///
    /// ## Returning a single field
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// use diesel::connection::DefaultLoadingMode;
    ///
    /// let data = users.select(name)
    ///     .load_iter::<String, DefaultLoadingMode>(connection)?
    ///     .collect::<QueryResult<Vec<_>>>()?;
    /// assert_eq!(vec!["Sean", "Tess"], data);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// ## Returning a tuple
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// use diesel::connection::DefaultLoadingMode;
    ///
    /// let data = users
    ///     .load_iter::<(i32, String), DefaultLoadingMode>(connection)?
    ///     .collect::<QueryResult<Vec<_>>>()?;
    /// let expected_data = vec![
    ///     (1, String::from("Sean")),
    ///     (2, String::from("Tess")),
    /// ];
    /// assert_eq!(expected_data, data);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// ## Returning a struct
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// #[derive(Queryable, PartialEq, Debug)]
    /// struct User {
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// use diesel::connection::DefaultLoadingMode;
    ///
    /// let data = users
    ///     .load_iter::<User, DefaultLoadingMode>(connection)?
    ///     .collect::<QueryResult<Vec<_>>>()?;
    /// let expected_data = vec![
    ///     User { id: 1, name: String::from("Sean") },
    ///     User { id: 2, name: String::from("Tess") },
    /// ];
    /// assert_eq!(expected_data, data);
    /// #     Ok(())
    /// # }
    /// ```
    fn load_iter<'conn, 'query: 'conn, U, B>(
        self,
        conn: &'conn mut Conn,
    ) -> QueryResult<Self::RowIter<'conn>>
    where
        U: 'conn,
        Self: LoadQuery<'query, Conn, U, B> + 'conn,
    {
        self.internal_load(conn)
    }

    /// Runs the command, and returns the affected row.
    ///
    /// `Err(NotFound)` will be returned if the query affected 0 rows. You can
    /// call `.optional()` on the result of this if the command was optional to
    /// get back a `Result<Option<U>>`
    ///
    /// When this method is called on an insert, update, or delete statement,
    /// it will implicitly add a `RETURNING *` to the query,
    /// unless a returning clause was already specified.
    ///
    /// This method only returns the first row that was affected, even if more
    /// rows are affected.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::{insert_into, update};
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let inserted_row = insert_into(users)
    ///     .values(name.eq("Ruby"))
    ///     .get_result(connection)?;
    /// assert_eq!((3, String::from("Ruby")), inserted_row);
    ///
    /// // This will return `NotFound`, as there is no user with ID 4
    /// let update_result = update(users.find(4))
    ///     .set(name.eq("Jim"))
    ///     .get_result::<(i32, String)>(connection);
    /// assert_eq!(Err(diesel::NotFound), update_result);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn get_result<'query, U>(self, conn: &mut Conn) -> QueryResult<U>
    where
        Self: LoadQuery<'query, Conn, U>,
    {
        match self.internal_load(conn)?.next() {
            Some(v) => v,
            None => Err(crate::result::Error::NotFound),
        }
    }

    /// Runs the command, returning an `Vec` with the affected rows.
    ///
    /// This method is an alias for [`load`], but with a name that makes more
    /// sense for insert, update, and delete statements.
    ///
    /// [`load`]: crate::query_dsl::RunQueryDsl::load()
    fn get_results<'query, U>(self, conn: &mut Conn) -> QueryResult<Vec<U>>
    where
        Self: LoadQuery<'query, Conn, U>,
    {
        self.load(conn)
    }

    /// Attempts to load a single record.
    ///
    /// This method is equivalent to `.limit(1).get_result()`
    ///
    /// Returns `Ok(record)` if found, and `Err(NotFound)` if no results are
    /// returned. If the query truly is optional, you can call `.optional()` on
    /// the result of this to get a `Result<Option<U>>`.
    ///
    /// # Example:
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// diesel::insert_into(users)
    ///     .values(&vec![name.eq("Sean"), name.eq("Pascal")])
    ///     .execute(connection)?;
    ///
    /// let first_name = users.order(id).select(name).first(connection);
    /// assert_eq!(Ok(String::from("Sean")), first_name);
    ///
    /// let not_found = users
    ///     .filter(name.eq("Foo"))
    ///     .first::<(i32, String)>(connection);
    /// assert_eq!(Err(diesel::NotFound), not_found);
    /// #     Ok(())
    /// # }
    /// ```
    fn first<'query, U>(self, conn: &mut Conn) -> QueryResult<U>
    where
        Self: methods::LimitDsl,
        Limit<Self>: LoadQuery<'query, Conn, U>,
    {
        methods::LimitDsl::limit(self, 1).get_result(conn)
    }
}

// Note: We could have a blanket `AsQuery` impl here, which would apply to
// everything we want it to. However, when a query is invalid, we specifically
// want the error to happen on the where clause of the method instead of trait
// resolution. Otherwise our users will get an error saying `<3 page long type>:
// ExecuteDsl is not satisfied` instead of a specific error telling them what
// part of their query is wrong.
impl<T, Conn> RunQueryDsl<Conn> for T where T: Table {}
