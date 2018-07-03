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
//! [expression_methods]: ../expression_methods/index.html
//! [dsl]: ../dsl/index.html
//! [`QueryDsl`]: trait.QueryDsl.html
//! [`RunQueryDsl`]: trait.RunQueryDsl.html

use backend::Backend;
use connection::Connection;
use expression::count::CountStar;
use expression::Expression;
use helper_types::*;
use query_builder::locking_clause as lock;
use query_source::{joins, Table};
use result::{first_or_not_found, QueryResult};

#[cfg(diesel_experimental)]
mod aliased_dsl;
mod belonging_to_dsl;
#[doc(hidden)]
pub mod boxed_dsl;
mod distinct_dsl;
#[doc(hidden)]
pub mod filter_dsl;
mod group_by_dsl;
mod join_dsl;
#[doc(hidden)]
pub mod limit_dsl;
#[doc(hidden)]
pub mod load_dsl;
mod locking_dsl;
mod offset_dsl;
mod order_dsl;
mod save_changes_dsl;
#[doc(hidden)]
pub mod select_dsl;
mod single_value_dsl;

pub use self::belonging_to_dsl::BelongingToDsl;
#[doc(hidden)]
pub use self::group_by_dsl::GroupByDsl;
pub use self::join_dsl::{InternalJoinDsl, JoinOnDsl, JoinWithImplicitOnClause};
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
    #[cfg(diesel_experimental)]
    pub use super::aliased_dsl::AliasedDsl;
    pub use super::boxed_dsl::BoxedDsl;
    pub use super::distinct_dsl::*;
    #[doc(inline)]
    pub use super::filter_dsl::*;
    pub use super::limit_dsl::LimitDsl;
    pub use super::load_dsl::{ExecuteDsl, LoadQuery};
    #[cfg(feature = "with-deprecated")]
    #[allow(deprecated)]
    pub use super::locking_dsl::ForUpdateDsl;
    pub use super::locking_dsl::{LockingDsl, ModifyLockDsl};
    pub use super::offset_dsl::OffsetDsl;
    pub use super::order_dsl::{OrderDsl, ThenOrderDsl};
    pub use super::select_dsl::SelectDsl;
    pub use super::single_value_dsl::SingleValueDsl;
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// #     connection.execute("DELETE FROM users").unwrap();
    /// diesel::insert_into(users)
    ///     .values(&vec![name.eq("Sean"); 3])
    ///     .execute(&connection)?;
    /// let names = users.select(name).load::<String>(&connection)?;
    /// let distinct_names = users.select(name).distinct().load::<String>(&connection)?;
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
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = establish_connection();
    /// #     connection.execute("DELETE FROM animals").unwrap();
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("dog"), name.eq(Some("Jack")), legs.eq(4)),
    ///         (species.eq("dog"), name.eq(None), legs.eq(4)),
    ///         (species.eq("spider"), name.eq(None), legs.eq(8)),
    ///     ])
    ///     .execute(&connection)
    ///     .unwrap();
    /// let all_animals = animals.select((species, name, legs)).load(&connection);
    /// let distinct_animals = animals.select((species, name, legs)).distinct_on(species).load(&connection);
    ///
    /// assert_eq!(Ok(vec![Animal::new("dog", Some("Jack"), 4),
    ///                    Animal::new("dog", None, 4),
    ///                    Animal::new("spider", None, 8)]), all_animals);
    /// assert_eq!(Ok(vec![Animal::new("dog", Some("Jack"), 4),
    ///                    Animal::new("spider", None, 8)]), distinct_animals);
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
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
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let connection = establish_connection();
    /// // By default, all columns will be selected
    /// let all_users = users.load::<(i32, String)>(&connection)?;
    /// assert_eq!(vec![(1, String::from("Sean")), (2, String::from("Tess"))], all_users);
    ///
    /// let all_names = users.select(name).load::<String>(&connection)?;
    /// assert_eq!(vec!["Sean", "Tess"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// ### When used with a left join
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = establish_connection();
    /// #     connection.execute("DELETE FROM posts")?;
    /// #     diesel::insert_into(posts::table)
    /// #         .values((posts::user_id.eq(1), posts::title.eq("Sean's Post")))
    /// #         .execute(&connection)?;
    /// #     let post_id = posts::table.select(posts::id)
    /// #         .first::<i32>(&connection)?;
    /// let join = users::table.left_join(posts::table);
    ///
    /// // By default, all columns from both tables are selected
    /// let all_data = join.load::<(User, Option<Post>)>(&connection)?;
    /// let expected_data = vec![
    ///     (User::new(1, "Sean"), Some(Post::new(post_id, 1, "Sean's Post"))),
    ///     (User::new(2, "Tess"), None),
    /// ];
    /// assert_eq!(expected_data, all_data);
    ///
    /// // Since `posts` is on the right side of a left join, `.nullable` is
    /// // needed.
    /// let names_and_titles = join.select((users::name, posts::title.nullable()))
    ///     .load::<(String, Option<String>)>(&connection)?;
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let count = users.count().get_result(&connection);
    /// assert_eq!(Ok(2), count);
    /// # }
    /// ```
    fn count(self) -> Select<Self, CountStar>
    where
        Self: methods::SelectDsl<CountStar>,
    {
        use dsl::count_star;

        QueryDsl::select(self, count_star())
    }

    /// Join two tables using a SQL `INNER JOIN`.
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
    /// You will also need to call [`allow_tables_to_appear_in_same_query!`].
    /// If you are using `infer_schema!` or `diesel print-schema`, this will
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
    ///
    /// # Examples
    ///
    /// ### With implicit `ON` clause
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = establish_connection();
    /// let data = users.inner_join(posts)
    ///     .select((name, title))
    ///     .load(&connection);
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
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = establish_connection();
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         (user_id.eq(1), title.eq("Sean's post")),
    ///         (user_id.eq(2), title.eq("Sean is a jerk")),
    ///     ])
    ///     .execute(&connection)
    ///     .unwrap();
    ///
    /// let data = users
    ///     .inner_join(posts.on(title.like(name.concat("%"))))
    ///     .select((name, title))
    ///     .load(&connection);
    /// let expected_data = vec![
    ///     (String::from("Sean"), String::from("Sean's post")),
    ///     (String::from("Sean"), String::from("Sean is a jerk")),
    /// ];
    /// assert_eq!(Ok(expected_data), data);
    /// # }
    /// ```
    fn inner_join<Rhs>(self, rhs: Rhs) -> Self::Output
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
    /// [`inner_join`]: #method.inner_join
    fn left_outer_join<Rhs>(self, rhs: Rhs) -> Self::Output
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::LeftOuter>,
    {
        self.join_with_implicit_on_clause(rhs, joins::LeftOuter)
    }

    /// Alias for [`left_outer_join`].
    ///
    /// [`left_outer_join`]: #method.left_outer_join
    fn left_join<Rhs>(self, rhs: Rhs) -> Self::Output
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::LeftOuter>,
    {
        self.left_outer_join(rhs)
    }

    /// Adds to the `WHERE` clause of a query.
    ///
    /// If there is already a `WHERE` clause, the result will be `old AND new`.
    ///
    /// # Example:
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let seans_id = users.filter(name.eq("Sean")).select(id)
    ///     .first(&connection);
    /// assert_eq!(Ok(1), seans_id);
    /// let tess_id = users.filter(name.eq("Tess")).select(id)
    ///     .first(&connection);
    /// assert_eq!(Ok(2), tess_id);
    /// # }
    /// ```
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// #     diesel::delete(animals).execute(&connection)?;
    /// diesel::insert_into(animals)
    ///     .values(&vec![
    ///         (species.eq("cat"), legs.eq(4), name.eq("Sinatra")),
    ///         (species.eq("dog"), legs.eq(3), name.eq("Fido")),
    ///         (species.eq("spider"), legs.eq(8), name.eq("Charlotte")),
    ///     ])
    ///     .execute(&connection)?;
    ///
    /// let good_animals = animals
    ///     .filter(name.eq("Fido"))
    ///     .or_filter(legs.eq(4))
    ///     .select(name)
    ///     .get_results::<Option<String>>(&connection)?;
    /// let expected = vec![
    ///     Some(String::from("Sinatra")),
    ///     Some(String::from("Fido")),
    /// ];
    /// assert_eq!(expected, good_animals);
    /// #     Ok(())
    /// # }
    /// ```
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     use diesel::result::Error::NotFound;
    /// #     let connection = establish_connection();
    /// let sean = (1, "Sean".to_string());
    /// let tess = (2, "Tess".to_string());
    /// assert_eq!(Ok(sean), users.find(1).first(&connection));
    /// assert_eq!(Ok(tess), users.find(2).first(&connection));
    /// assert_eq!(Err::<(i32, String), _>(NotFound), users.find(3).first(&connection));
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
    /// If there was already a order clause, it will be overridden. See
    /// also:
    /// [`.desc()`](../expression_methods/trait.ExpressionMethods.html#method.desc)
    /// and
    /// [`.asc()`](../expression_methods/trait.ExpressionMethods.html#method.asc)
    ///
    /// Ordering by multiple columns can be achieved by passing a tuple of those
    /// columns.
    /// To construct an order clause of an unknown number of columns,
    /// see [`QueryDsl::then_order_by`](#method.then_order_by)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// #     connection.execute("DELETE FROM users")?;
    /// diesel::insert_into(users)
    ///     .values(&vec![
    ///         name.eq("Saul"),
    ///         name.eq("Steve"),
    ///         name.eq("Stan"),
    ///     ])
    ///     .execute(&connection)?;
    ///
    /// let ordered_names = users.select(name)
    ///     .order(name.desc())
    ///     .load::<String>(&connection)?;
    /// assert_eq!(vec!["Steve", "Stan", "Saul"], ordered_names);
    ///
    /// diesel::insert_into(users).values(name.eq("Stan")).execute(&connection)?;
    ///
    /// let data = users.select((name, id))
    ///     .order((name.asc(), id.desc()))
    ///     .load(&connection)?;
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
    fn order_by<Expr>(self, expr: Expr) -> Order<Self, Expr>
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// #     connection.execute("DELETE FROM users")?;
    /// diesel::insert_into(users)
    ///     .values(&vec![
    ///         name.eq("Saul"),
    ///         name.eq("Steve"),
    ///         name.eq("Stan"),
    ///         name.eq("Stan"),
    ///     ])
    ///     .execute(&connection)?;
    ///
    /// let data = users.select((name, id))
    ///     .order_by(name.asc())
    ///     .then_order_by(id.desc())
    ///     .load(&connection)?;
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use users::dsl::*;
    /// #     let connection = establish_connection();
    /// #     diesel::delete(users).execute(&connection)?;
    /// #     diesel::insert_into(users)
    /// #        .values(&vec![
    /// #            name.eq("Sean"),
    /// #            name.eq("Bastien"),
    /// #            name.eq("Pascal"),
    /// #        ])
    /// #        .execute(&connection)?;
    /// #
    /// // Using a limit
    /// let limited = users.select(name)
    ///     .order(id)
    ///     .limit(1)
    ///     .load::<String>(&connection)?;
    ///
    /// // Without a limit
    /// let no_limit = users.select(name)
    ///     .order(id)
    ///     .load::<String>(&connection)?;
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use users::dsl::*;
    /// #     let connection = establish_connection();
    /// #     diesel::delete(users).execute(&connection)?;
    /// #     diesel::insert_into(users)
    /// #        .values(&vec![
    /// #            name.eq("Sean"),
    /// #            name.eq("Bastien"),
    /// #            name.eq("Pascal"),
    /// #        ])
    /// #        .execute(&connection)?;
    /// #
    /// // Using an offset
    /// let offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .offset(1)
    ///     .load::<String>(&connection)?;
    ///
    /// // No Offset
    /// let no_offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .load::<String>(&connection)?;
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
    /// ```ignore
    /// // Executes `SELECT * FROM users FOR UPDATE`
    /// users.for_update().load(&connection)
    /// ```
    #[cfg(feature = "with-deprecated")]
    #[allow(deprecated)]
    fn for_update(self) -> ForUpdate<Self>
    where
        Self: methods::ForUpdateDsl,
    {
        methods::ForUpdateDsl::for_update(self)
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
    /// ```ignore
    /// // Executes `SELECT * FROM users FOR UPDATE`
    /// users.for_update().load(&connection)
    /// ```
    #[cfg(not(feature = "with-deprecated"))]
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
    /// ```ignore
    /// // Executes `SELECT * FROM users FOR NO KEY UPDATE`
    /// users.for_no_key_update().load(&connection)
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
    /// ```ignore
    /// // Executes `SELECT * FROM users FOR SHARE`
    /// users.for_share().load(&connection)
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
    /// ```ignore
    /// // Executes `SELECT * FROM users FOR KEY SHARE`
    /// users.for_key_share().load(&connection)
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
    /// ```ignore
    /// // Executes `SELECT * FROM users FOR UPDATE SKIP LOCKED`
    /// users.for_update().skip_locked().load(&connection)
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
    /// ```ignore
    /// // Executes `SELECT * FROM users FOR UPDATE NOWAIT`
    /// users.for_update().no_wait().load(&connection)
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     use std::collections::HashMap;
    /// #     let connection = establish_connection();
    /// #     let mut params = HashMap::new();
    /// #     params.insert("name", "Sean");
    /// let mut query = users::table.into_boxed();
    /// if let Some(name) = params.get("name") {
    ///     query = query.filter(users::name.eq(name));
    /// }
    /// let users = query.load(&connection);
    /// #     let expected = vec![(1, String::from("Sean"))];
    /// #     assert_eq!(Ok(expected), users);
    /// # }
    /// ```
    ///
    /// Diesel queries also have a similar problem to [`Iterator`][iterator], where
    /// returning them from a function requires exposing the implementation of that
    /// function. The [`helper_types`][helper_types] module exists to help with this,
    /// but you might want to hide the return type or have it conditionally change.
    /// Boxing can achieve both.
    ///
    /// [iterator]: https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html
    /// [helper_types]: ../helper_types/index.html
    ///
    /// ### Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::users;
    /// #
    /// # fn main() {
    /// #     let connection = establish_connection();
    /// fn users_by_name<'a>(name: &'a str) -> users::BoxedQuery<'a, DB> {
    ///     users::table.filter(users::name.eq(name)).into_boxed()
    /// }
    ///
    /// assert_eq!(Ok(1), users_by_name("Sean").select(users::id).first(&connection));
    /// assert_eq!(Ok(2), users_by_name("Tess").select(users::id).first(&connection));
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
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = establish_connection();
    /// insert_into(posts::table)
    ///     .values(posts::user_id.eq(1))
    ///     .execute(&connection)?;
    /// let last_post = posts::table
    ///     .order(posts::id.desc());
    /// let most_recently_active_user = users.select(name)
    ///     .filter(id.nullable().eq(last_post.select(posts::user_id).single_value()))
    ///     .first::<String>(&connection)?;
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

    #[cfg(diesel_experimental)]
    /// Aliases the query, allowing tables to appear more than once in the same query or joining to
    /// a subselect
    ///
    /// FIXME: Examples for joining to the same table twice, joining to subselects, self
    /// referential subselects
    fn aliased<T>(self, alias: T) -> Aliased<Self, T>
    where
        Self: methods::AliasedDsl<T>,
    {
        methods::AliasedDsl::aliased(self, alias)
    }
}

impl<T: Table> QueryDsl for T {}

/// Methods used to execute queries.
pub trait RunQueryDsl<Conn>: Sized {
    /// Executes the given command, returning the number of rows affected.
    ///
    /// `execute` is usually used in conjunction with [`insert_into`](../fn.insert_into.html),
    /// [`update`](../fn.update.html) and [`delete`](../fn.delete.html) where the number of
    /// affected rows is often enough information.
    ///
    /// When asking the database to return data from a query, [`load`](#method.load) should
    /// probably be used instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let inserted_rows = insert_into(users)
    ///     .values(name.eq("Ruby"))
    ///     .execute(&connection)?;
    /// assert_eq!(1, inserted_rows);
    ///
    /// let inserted_rows = insert_into(users)
    ///     .values(&vec![name.eq("Jim"), name.eq("James")])
    ///     .execute(&connection)?;
    /// assert_eq!(2, inserted_rows);
    /// #     Ok(())
    /// # }
    /// ```
    fn execute(self, conn: &Conn) -> QueryResult<usize>
    where
        Conn: Connection,
        Self: methods::ExecuteDsl<Conn>,
    {
        methods::ExecuteDsl::execute(self, conn)
    }

    /// Executes the given query, returning a `Vec` with the returned rows.
    ///
    /// When using the query builder,
    /// the return type can be
    /// a tuple of the values,
    /// or a struct which implements [`Queryable`].
    ///
    /// When this method is called on [`sql_query`],
    /// the return type can only be a struct which implements [`QueryableByName`]
    ///
    /// For insert, update, and delete operations where only a count of affected is needed,
    /// [`execute`] should be used instead.
    ///
    /// [`Queryable`]: ../deserialize/trait.Queryable.html
    /// [`QueryableByName`]: ../deserialize/trait.QueryableByName.html
    /// [`execute`]: fn.execute.html
    /// [`sql_query`]: ../fn.sql_query.html
    ///
    /// # Examples
    ///
    /// ## Returning a single field
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let data = users.select(name)
    ///     .load::<String>(&connection)?;
    /// assert_eq!(vec!["Sean", "Tess"], data);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// ## Returning a tuple
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::insert_into;
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let data = users
    ///     .load::<(i32, String)>(&connection)?;
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
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = establish_connection();
    /// let data = users
    ///     .load::<User>(&connection)?;
    /// let expected_data = vec![
    ///     User { id: 1, name: String::from("Sean"), },
    ///     User { id: 2, name: String::from("Tess"), },
    /// ];
    /// assert_eq!(expected_data, data);
    /// #     Ok(())
    /// # }
    /// ```
    fn load<U>(self, conn: &Conn) -> QueryResult<Vec<U>>
    where
        Self: LoadQuery<Conn, U>,
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
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = establish_connection();
    /// let inserted_row = insert_into(users)
    ///     .values(name.eq("Ruby"))
    ///     .get_result(&connection)?;
    /// assert_eq!((3, String::from("Ruby")), inserted_row);
    ///
    /// // This will return `NotFound`, as there is no user with ID 4
    /// let update_result = update(users.find(4))
    ///     .set(name.eq("Jim"))
    ///     .get_result::<(i32, String)>(&connection);
    /// assert_eq!(Err(diesel::NotFound), update_result);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "postgres"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn get_result<U>(self, conn: &Conn) -> QueryResult<U>
    where
        Self: LoadQuery<Conn, U>,
    {
        first_or_not_found(self.load(conn))
    }

    /// Runs the command, returning an `Vec` with the affected rows.
    ///
    /// This method is an alias for [`load`], but with a name that makes more
    /// sense for insert, update, and delete statements.
    ///
    /// [`load`]: #method.load
    fn get_results<U>(self, conn: &Conn) -> QueryResult<Vec<U>>
    where
        Self: LoadQuery<Conn, U>,
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// diesel::insert_into(users)
    ///     .values(&vec![name.eq("Sean"), name.eq("Pascal")])
    ///     .execute(&connection)?;
    ///
    /// let first_name = users.order(id).select(name).first(&connection);
    /// assert_eq!(Ok(String::from("Sean")), first_name);
    ///
    /// let not_found = users
    ///     .filter(name.eq("Foo"))
    ///     .first::<(i32, String)>(&connection);
    /// assert_eq!(Err(diesel::NotFound), not_found);
    /// #     Ok(())
    /// # }
    /// ```
    fn first<U>(self, conn: &Conn) -> QueryResult<U>
    where
        Self: methods::LimitDsl,
        Limit<Self>: LoadQuery<Conn, U>,
    {
        methods::LimitDsl::limit(self, 1).get_result(conn)
    }
}

// Note: We could have a blanket `AsQuery` impl here, which would apply to
// everything we want it to. However, the entire point of this trait is to have
// trait resolution succeed, and the where clause on the methods fail when the
// query is invalid. So we need things to unconditionally implement this trait.
impl<T, Conn> RunQueryDsl<Conn> for T
where
    T: Table,
{
}
