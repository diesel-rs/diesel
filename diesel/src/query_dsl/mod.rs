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
use expression::Expression;
use expression::count::CountStar;
use helper_types::*;
use query_source::{joins, Table};
use result::{first_or_not_found, QueryResult};

mod belonging_to_dsl;
#[doc(hidden)]
pub mod boxed_dsl;
mod distinct_dsl;
mod group_by_dsl;
mod join_dsl;
#[doc(hidden)]
pub mod limit_dsl;
#[doc(hidden)]
pub mod load_dsl;
mod locking_dsl;
#[doc(hidden)]
pub mod select_dsl;
#[doc(hidden)]
pub mod filter_dsl;
mod save_changes_dsl;
mod offset_dsl;
mod order_dsl;

pub use self::belonging_to_dsl::BelongingToDsl;
pub use self::boxed_dsl::BoxedDsl;
#[doc(hidden)]
pub use self::group_by_dsl::GroupByDsl;
pub use self::join_dsl::{InternalJoinDsl, JoinOnDsl, JoinWithImplicitOnClause};
pub use self::load_dsl::LoadQuery;
pub use self::save_changes_dsl::SaveChangesDsl;

/// The traits used by `QueryDsl`.
///
/// Each trait in this module represents exactly one method from `QueryDsl`.
/// Apps should general rely on `QueryDsl` directly, rather than these traits.
/// However, generic code may need to include a where clause that references
/// these traits.
pub mod methods {
    pub use super::distinct_dsl::*;
    #[doc(inline)]
    pub use super::filter_dsl::*;
    pub use super::limit_dsl::LimitDsl;
    pub use super::load_dsl::ExecuteDsl;
    pub use super::locking_dsl::ForUpdateDsl;
    pub use super::offset_dsl::OffsetDsl;
    pub use super::order_dsl::OrderDsl;
    pub use super::select_dsl::SelectDsl;
}

pub trait QueryDsl: Sized {
    /// Adds the `DISTINCT` keyword to a query.
    ///
    /// # Example
    ///
    /// ```rust
    ///
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
    /// #     use self::users::dsl::*;
    /// #     let connection = establish_connection();
    /// #     connection.execute("DELETE FROM users").unwrap();
    /// connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Sean'), ('Sean')")
    ///     .unwrap();
    /// let names = users.select(name).load(&connection);
    /// let distinct_names = users.select(name).distinct().load(&connection);
    ///
    /// let sean = String::from("Sean");
    /// assert_eq!(Ok(vec![sean.clone(), sean.clone(), sean.clone()]), names);
    /// assert_eq!(Ok(vec![sean.clone()]), distinct_names);
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
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
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
    /// Sets the select clause of a query.
    ///
    /// If there was already a select clause, it will be overridden. The
    /// expression passed to `select` must actually be valid for the query (only
    /// contains columns from the target table, doesn't mix aggregate +
    /// non-aggregate expressions, etc).
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
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
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
    fn inner_join<Rhs>(self, rhs: Rhs) -> Self::Output
    where
        Self: JoinWithImplicitOnClause<Rhs, joins::Inner>,
    {
        self.join_with_implicit_on_clause(rhs, joins::Inner)
    }

    /// Join two tables using a SQL `LEFT OUTER JOIN`. The `ON` clause is defined
    /// via the [associations API](../associations/index.html).
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
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
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

    /// Attempts to find a single record from the given table by primary key.
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
    /// #     use self::users::dsl::*;
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
    /// If there was already a order clause, it will be overridden. The
    /// expression passed to `order` must actually be valid for the query. See
    /// also:
    /// [`.desc()`](../expression_methods/trait.ExpressionMethods.html#method.desc)
    /// and
    /// [`.asc()`](../expression_methods/trait.ExpressionMethods.html#method.asc)
    ///
    /// Ordering by multiple columns can be achieved by passing a tuple of those
    /// columns.
    ///
    /// # Examples
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
    /// use self::users::dsl::{users, id, name};
    ///
    /// let connection = establish_connection();
    /// # connection.execute("DELETE FROM users").unwrap();
    /// connection.execute("INSERT INTO users (name) VALUES ('Saul'), ('Steve'), ('Stan')").unwrap();
    /// // load all users' names, ordered by their name descending
    /// let ordered_names: Vec<String> = users.select(name).order(name.desc()).load(&connection).unwrap();
    /// assert_eq!(vec![String::from("Steve"), String::from("Stan"), String::from("Saul")], ordered_names);
    ///
    /// connection.execute("INSERT INTO users (name) VALUES ('Stan')").unwrap();
    /// let ordered_name_id_pairs = users.select((name, id)).order((name.asc(), id.desc())).load(&connection).unwrap();
    /// assert_eq!(vec![(String::from("Saul"), 3), (String::from("Stan"), 6), (String::from("Stan"), 5), (String::from("Steve"), 4)], ordered_name_id_pairs);
    /// # }
    /// ```
    fn order<Expr>(self, expr: Expr) -> Order<Self, Expr>
    where
        Expr: Expression,
        Self: methods::OrderDsl<Expr>,
    {
        methods::OrderDsl::order(self, expr)
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
    /// #   use users::dsl::*;
    /// #   let connection = establish_connection();
    /// #   diesel::delete(users).execute(&connection).unwrap();
    /// #
    /// # let new_users = vec![
    /// #    NewUser { name: "Sean".to_string(), },
    /// #    NewUser { name: "Bastien".to_string(), },
    /// #    NewUser { name: "Pascal".to_string(), },
    /// # ];
    /// #
    /// # diesel::insert_into(users)
    /// #    .values(&new_users)
    /// #    .execute(&connection)
    /// #    .unwrap();
    /// #
    /// // Using a limit
    /// let limited = users.select(name)
    ///     .order(id)
    ///     .limit(1)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// // Without a limit
    /// let no_limit = users.select(name)
    ///     .order(id)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// assert_eq!(vec!["Sean".to_string()], limited);
    /// assert_eq!(vec!["Sean".to_string(), "Bastien".to_string(), "Pascal".to_string()], no_limit);
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
    /// #   use users::dsl::*;
    /// #   let connection = establish_connection();
    /// #   diesel::delete(users).execute(&connection).unwrap();
    /// #
    /// # let new_users = vec![
    /// #    NewUser { name: "Sean".to_string(), },
    /// #    NewUser { name: "Bastien".to_string(), },
    /// #    NewUser { name: "Pascal".to_string(), },
    /// # ];
    /// #
    /// # diesel::insert_into(users)
    /// #    .values(&new_users)
    /// #    .execute(&connection)
    /// #    .unwrap();
    /// #
    /// // Using an offset
    /// let offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .offset(1)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// // No Offset
    /// let no_offset = users.select(name)
    ///     .order(id)
    ///     .limit(2)
    ///     .load::<String>(&connection)
    ///     .unwrap();
    ///
    /// assert_eq!(vec!["Bastien".to_string(), "Pascal".to_string()], offset);
    /// assert_eq!(vec!["Sean".to_string(), "Bastien".to_string()], no_offset);
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
    fn for_update(self) -> ForUpdate<Self>
    where
        Self: methods::ForUpdateDsl,
    {
        methods::ForUpdateDsl::for_update(self)
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
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
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
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #     }
    /// # }
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
        Self: BoxedDsl<'a, DB>,
    {
        self.internal_into_boxed()
    }
}

impl<T: Table> QueryDsl for T {}

pub trait RunQueryDsl<Conn>: Sized {
    /// Executes the given command, returning the number of rows affected.
    ///
    /// Used in conjunction with [`insert_into`](../fn.insert_into.html),
    /// [`update`](../fn.update.html) and [`delete`](../fn.delete.html)
    fn execute(self, conn: &Conn) -> QueryResult<usize>
    where
        Conn: Connection,
        Self: methods::ExecuteDsl<Conn>,
    {
        methods::ExecuteDsl::execute(self, conn)
    }

    /// Executes the given query, returning a `Vec` with the returned rows.
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
    fn get_result<U>(self, conn: &Conn) -> QueryResult<U>
    where
        Self: LoadQuery<Conn, U>,
    {
        first_or_not_found(self.load(conn))
    }

    /// Runs the command, returning an `Vec` with the affected rows.
    fn get_results<U>(self, conn: &Conn) -> QueryResult<Vec<U>>
    where
        Self: LoadQuery<Conn, U>,
    {
        self.load(conn)
    }

    /// Attempts to load a single record.
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
    /// # use diesel::NotFound;
    /// table! {
    ///     users {
    ///         id -> Integer,
    ///         name -> VarChar,
    ///     }
    /// }
    ///
    /// #[derive(Queryable, PartialEq, Debug)]
    /// struct User {
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// # fn main() {
    /// #   let connection = establish_connection();
    /// let user1 = NewUser { name: "Sean".into() };
    /// let user2 = NewUser { name: "Pascal".into() };
    /// diesel::insert_into(users::table).values(&vec![user1, user2]).execute(&connection).unwrap();
    ///
    /// let user = users::table.order(users::id.asc()).first(&connection);
    /// assert_eq!(Ok(User { id: 1, name: "Sean".into() }), user);
    /// let user = users::table.filter(users::name.eq("Foo")).first::<User>(&connection);
    /// assert_eq!(Err(NotFound), user);
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
