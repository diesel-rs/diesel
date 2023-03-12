use crate::expression::Expression;
use crate::query_builder::upsert::into_conflict_clause::IntoConflictValueClause;
use crate::query_builder::upsert::on_conflict_actions::*;
use crate::query_builder::upsert::on_conflict_clause::*;
use crate::query_builder::upsert::on_conflict_target::*;
pub use crate::query_builder::upsert::on_conflict_target_decorations::DecoratableTarget;
use crate::query_builder::where_clause::{NoWhereClause, WhereAnd, WhereOr};
use crate::query_builder::{AsChangeset, InsertStatement, UndecoratedInsertRecord};
use crate::query_dsl::filter_dsl::FilterDsl;
use crate::query_dsl::methods::OrFilterDsl;
use crate::query_source::QuerySource;
use crate::sql_types::BoolOrNullableBool;

impl<T, U, Op, Ret> InsertStatement<T, U, Op, Ret>
where
    T: QuerySource,
    U: UndecoratedInsertRecord<T> + IntoConflictValueClause,
{
    /// Adds `ON CONFLICT DO NOTHING` to the insert statement, without
    /// specifying any columns or constraints to restrict the conflict to.
    ///
    /// # Examples
    ///
    /// ### Single Record
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap()
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     #[cfg(feature = "postgres")]
    /// #     diesel::sql_query("TRUNCATE TABLE users").execute(conn).unwrap();
    /// #     #[cfg(any(feature = "sqlite", feature = "mysql"))]
    /// #     diesel::sql_query("DELETE FROM users").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Sean" };
    ///
    /// let user_count = users.count().get_result::<i64>(conn)?;
    /// assert_eq!(user_count, 0);
    ///
    /// diesel::insert_into(users)
    ///     .values(&user)
    ///     .on_conflict_do_nothing()
    ///     .execute(conn)?;
    /// let user_count = users.count().get_result::<i64>(conn)?;
    /// assert_eq!(user_count, 1);
    ///
    /// diesel::insert_into(users)
    ///     .values(&user)
    ///     .on_conflict_do_nothing()
    ///     .execute(conn)?;
    /// let user_count = users.count().get_result::<i64>(conn)?;
    /// assert_eq!(user_count, 1);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ### Vec of Records
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap()
    /// # }
    /// #
    /// # fn run_test() -> diesel::QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     #[cfg(feature = "postgres")]
    /// #     diesel::sql_query("TRUNCATE TABLE users").execute(conn).unwrap();
    /// #     #[cfg(any(feature = "mysql", feature = "sqlite"))]
    /// #     diesel::sql_query("DELETE FROM users").execute(conn).unwrap();
    /// # #[cfg(any(feature = "postgres", feature = "mysql"))]
    /// let user = User { id: 1, name: "Sean" };
    ///
    /// # #[cfg(any(feature = "postgres", feature = "mysql"))]
    /// let inserted_row_count = diesel::insert_into(users)
    ///     .values(&vec![user, user])
    ///     .on_conflict_do_nothing()
    ///     .execute(conn)?;
    /// # #[cfg(any(feature = "postgres", feature = "mysql"))]
    /// let user_count = users.count().get_result::<i64>(conn)?;
    /// # #[cfg(any(feature = "postgres", feature = "mysql"))]
    /// assert_eq!(user_count, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn on_conflict_do_nothing(
        self,
    ) -> InsertStatement<T, OnConflictValues<U::ValueClause, NoConflictTarget, DoNothing<T>>, Op, Ret>
    {
        self.replace_values(|values| OnConflictValues::do_nothing(values.into_value_clause()))
    }

    /// Adds an `ON CONFLICT` to the insert statement, if a conflict occurs
    /// for the given unique constraint.
    ///
    /// `Target` can be one of:
    ///
    /// - A column
    /// - A tuple of columns
    /// - [`on_constraint("constraint_name")`][`on_constraint`]
    ///
    /// # Examples
    ///
    /// ### Specifying a column as the target
    ///
    /// This is supported by sqlite and postgres only
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #         hair_color -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Clone, Copy, Insertable)]
    /// # #[diesel(table_name = users)]
    /// # struct User<'a> {
    /// #     id: i32,
    /// #     name: &'a str,
    /// # }
    /// #
    /// # fn main() {
    /// #    run_test().unwrap()
    /// # }
    /// # #[cfg(any(feature = "postgres", feature = "sqlite"))]
    /// # fn run_test() -> diesel::QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// use diesel::upsert::*;
    ///
    /// #     let conn = &mut establish_connection();
    /// #     #[cfg(any(feature = "sqlite", feature = "postgres"))]
    /// #     diesel::sql_query("DROP TABLE users").execute(conn).unwrap();
    /// #     #[cfg(any(feature = "sqlite", feature = "postgres"))]
    /// #     diesel::sql_query("CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)").execute(conn).unwrap();
    /// diesel::sql_query("CREATE UNIQUE INDEX users_name ON users (name)").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Sean" };
    /// let same_name_different_id = User { id: 2, name: "Sean" };
    /// let same_id_different_name = User { id: 1, name: "Pascal" };

    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// let query = diesel::insert_into(users)
    ///     .values(&same_id_different_name)
    ///     .on_conflict(id)
    ///     .do_nothing()
    ///     .execute(conn)?;
    ///
    /// let user_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(user_names, vec![String::from("Sean")]);
    ///
    /// let idx_conflict_result = diesel::insert_into(users)
    ///     .values(&same_name_different_id)
    ///     .on_conflict(id)
    ///     .do_nothing()
    ///     .execute(conn);
    /// assert!(idx_conflict_result.is_err());
    /// # Ok(())
    /// # }
    /// #[cfg(feature = "mysql")]
    /// fn run_test() -> diesel::QueryResult<()> { Ok(()) }
    /// ```
    ///
    /// ### Specifying multiple columns as the target
    ///
    /// This is supported by sqlite and postgres only
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #         hair_color -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Clone, Copy, Insertable)]
    /// # #[diesel(table_name = users)]
    /// # struct User<'a> {
    /// #     id: i32,
    /// #     name: &'a str,
    /// #     hair_color: &'a str,
    /// # }
    /// #
    /// # #[cfg(any(feature = "sqlite", feature = "postgres"))]
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// use diesel::upsert::*;
    ///
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE users").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT, hair_color TEXT)").execute(conn).unwrap();
    /// diesel::sql_query("CREATE UNIQUE INDEX users_name_hair_color ON users (name, hair_color)").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Sean", hair_color: "black" };
    /// let same_name_different_hair_color = User { id: 2, name: "Sean", hair_color: "brown" };
    /// let same_name_same_hair_color = User { id: 3, name: "Sean", hair_color: "black" };
    ///
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// let inserted_row_count = diesel::insert_into(users)
    ///     .values(&same_name_different_hair_color)
    ///     .on_conflict((name, hair_color))
    ///     .do_nothing()
    ///     .execute(conn);
    /// assert_eq!(Ok(1), inserted_row_count);
    ///
    /// let inserted_row_count = diesel::insert_into(users)
    ///     .values(&same_name_same_hair_color)
    ///     .on_conflict((name, hair_color))
    ///     .do_nothing()
    ///     .execute(conn);
    /// assert_eq!(Ok(0), inserted_row_count);
    /// # }
    ///
    /// #[cfg(feature = "mysql")]
    /// fn main() {}
    /// ```
    ///
    /// ### ON DUPLICATE KEY
    ///
    /// Mysql supports only catching all duplicated keys at once:
    ///
    /// ```
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> VarChar,
    /// #         hair_color -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # #[derive(Clone, Copy, Insertable)]
    /// # #[diesel(table_name = users)]
    /// # struct User<'a> {
    /// #     id: i32,
    /// #     name: &'a str,
    /// # }
    /// #
    /// # fn main() {
    /// #    run_test().unwrap()
    /// # }
    /// # #[cfg(feature = "mysql")]
    /// # fn run_test() -> diesel::QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// use diesel::upsert::*;
    ///
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("CREATE TEMPORARY TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255), hair_color VARCHAR(255))").execute(conn).unwrap();
    /// diesel::sql_query("CREATE UNIQUE INDEX users_name ON users (name)").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Sean" };
    /// let same_name_different_id = User { id: 2, name: "Sean" };
    /// let same_id_different_name = User { id: 1, name: "Pascal" };
    ///
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// # diesel::delete(users.filter(name.ne("Sean"))).execute(conn)?;
    /// let user_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(user_names, vec![String::from("Sean")]);
    ///
    /// let query = diesel::insert_into(users)
    ///     .values(&same_id_different_name)
    ///     .on_conflict(diesel::dsl::DuplicatedKeys)
    ///     .do_nothing()
    ///     .execute(conn)?;
    ///
    /// let user_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(user_names, vec![String::from("Sean")]);
    ///
    /// let idx_conflict_result = diesel::insert_into(users)
    ///     .values(&same_name_different_id)
    ///     .on_conflict(diesel::dsl::DuplicatedKeys)
    ///     .do_nothing()
    ///     .execute(conn)?;
    ///
    /// let user_names = users.select(name).load::<String>(conn)?;
    /// assert_eq!(user_names, vec![String::from("Sean")]);
    /// # Ok(())
    /// # }
    /// #[cfg(not(feature = "mysql"))]
    /// fn run_test() -> diesel::QueryResult<()> {Ok(())}
    /// ```
    ///
    /// See the documentation for [`on_constraint`] and [`do_update`] for
    /// more examples.
    ///
    /// [`on_constraint`]: ../upsert/fn.on_constraint.html
    /// [`do_update`]: crate::upsert::IncompleteOnConflict::do_update()
    pub fn on_conflict<Target>(
        self,
        target: Target,
    ) -> IncompleteOnConflict<InsertStatement<T, U::ValueClause, Op, Ret>, ConflictTarget<Target>>
    where
        ConflictTarget<Target>: OnConflictTarget<T>,
    {
        IncompleteOnConflict {
            stmt: self.replace_values(IntoConflictValueClause::into_value_clause),
            target: ConflictTarget(target),
        }
    }
}

impl<Stmt, T, P> DecoratableTarget<P> for IncompleteOnConflict<Stmt, T>
where
    P: Expression,
    P::SqlType: BoolOrNullableBool,
    T: DecoratableTarget<P>,
{
    type FilterOutput = IncompleteOnConflict<Stmt, <T as DecoratableTarget<P>>::FilterOutput>;
    fn filter_target(self, predicate: P) -> Self::FilterOutput {
        IncompleteOnConflict {
            stmt: self.stmt,
            target: self.target.filter_target(predicate),
        }
    }
}

/// A partially constructed `ON CONFLICT` clause.
#[derive(Debug, Clone, Copy)]
pub struct IncompleteOnConflict<Stmt, Target> {
    stmt: Stmt,
    target: Target,
}

impl<T: QuerySource, U, Op, Ret, Target>
    IncompleteOnConflict<InsertStatement<T, U, Op, Ret>, Target>
{
    /// Creates a query with `ON CONFLICT (target) DO NOTHING`
    ///
    /// If you want to do nothing when *any* constraint conflicts, use
    /// [`on_conflict_do_nothing`] instead. See [`on_conflict`] for usage
    /// examples.
    ///
    /// [`on_conflict_do_nothing`]: crate::query_builder::InsertStatement::on_conflict_do_nothing()
    /// [`on_conflict`]: crate::query_builder::InsertStatement::on_conflict()
    pub fn do_nothing(
        self,
    ) -> InsertStatement<T, OnConflictValues<U, Target, DoNothing<T>>, Op, Ret> {
        let target = self.target;
        self.stmt.replace_values(|values| {
            OnConflictValues::new(values, target, DoNothing::new(), NoWhereClause)
        })
    }
}

impl<Stmt, Target> IncompleteOnConflict<Stmt, Target> {
    /// Used to create a query in the form `ON CONFLICT (...) DO UPDATE ... [WHERE ...]`
    ///
    /// Call `.set` on the result of this function with the changes you want to
    /// apply. The argument to `set` can be anything that implements `AsChangeset`
    /// (e.g. anything you could pass to `set` on a normal update statement).
    ///
    /// Note: When inserting more than one row at a time, this query can still fail
    /// if the rows being inserted conflict with each other.
    ///
    /// Some backends (PostgreSQL) support `WHERE` clause is used to limit the rows actually updated.
    /// For PostgreSQL you can use the `.filter()` method to add conditions like this.
    ///
    /// # Examples
    ///
    /// ## Set specific value on conflict
    ///
    /// PostgreSQL/SQLite:
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # #[cfg(not(feature = "mysql"))]
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     #[cfg(feature = "postgres")]
    /// #     diesel::sql_query("TRUNCATE TABLE users").execute(conn).unwrap();
    /// #     #[cfg(feature = "sqlite")]
    /// #     diesel::sql_query("DELETE FROM users").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Pascal" };
    /// let user2 = User { id: 1, name: "Sean" };
    ///
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// let insert_count = diesel::insert_into(users)
    ///     .values(&user2)
    ///     .on_conflict(id)
    ///     .do_update()
    ///     .set(name.eq("I DONT KNOW ANYMORE"))
    ///     .execute(conn);
    /// # #[cfg(any(feature = "sqlite", feature = "postgres"))]
    /// assert_eq!(Ok(1), insert_count);
    /// # #[cfg(feature = "mysql")]
    /// assert_eq!(Ok(2), insert_count);
    ///
    /// let users_in_db = users.load(conn);
    /// assert_eq!(Ok(vec![(1, "I DONT KNOW ANYMORE".to_string())]), users_in_db);
    /// # }
    /// # #[cfg(feature = "mysql")]
    /// # fn main() {}
    /// ```
    ///
    /// MySQL:
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # #[cfg(feature = "mysql")]
    /// # fn main() -> diesel::QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM users").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Pascal" };
    /// let user2 = User { id: 1, name: "Sean" };
    ///
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// diesel::insert_into(users)
    ///     .values(&user2)
    ///     .on_conflict(diesel::dsl::DuplicatedKeys)
    ///     .do_update()
    ///     .set(name.eq("I DONT KNOW ANYMORE"))
    ///     .execute(conn)?;
    ///
    /// let users_in_db = users.load(conn);
    /// assert_eq!(Ok(vec![(1, "I DONT KNOW ANYMORE".to_string())]), users_in_db);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "mysql"))]
    /// # fn main() {}
    /// ```
    ///
    /// ## Set `AsChangeset` struct on conflict
    ///
    /// PostgreSQL & SQLite:
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # #[cfg(not(feature = "mysql"))]
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     #[cfg(feature = "postgres")]
    /// #     diesel::sql_query("TRUNCATE TABLE users").execute(conn).unwrap();
    /// #     #[cfg(feature = "sqlite")]
    /// #     diesel::sql_query("DELETE FROM users").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Pascal" };
    /// let user2 = User { id: 1, name: "Sean" };
    ///
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// let insert_count = diesel::insert_into(users)
    ///     .values(&user2)
    ///     .on_conflict(id)
    ///     .do_update()
    ///     .set(&user2)
    ///     .execute(conn);
    /// assert_eq!(Ok(1), insert_count);
    ///
    /// let users_in_db = users.load(conn);
    /// assert_eq!(Ok(vec![(1, "Sean".to_string())]), users_in_db);
    /// # }
    /// # #[cfg(feature = "mysql")]
    /// # fn main() {}
    /// ```
    ///
    /// MySQL:
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    ///
    /// # #[cfg(feature = "mysql")]
    /// # fn main() -> diesel::QueryResult<()> {
    /// #     use self::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DELETE FROM users").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Pascal" };
    /// let user2 = User { id: 1, name: "Sean" };
    ///
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// diesel::insert_into(users)
    ///     .values(&user2)
    ///     .on_conflict(diesel::dsl::DuplicatedKeys)
    ///     .do_update()
    ///     .set(&user2)
    ///     .execute(conn)?;
    ///
    /// let users_in_db = users.load(conn);
    /// assert_eq!(Ok(vec![(1, "Sean".to_string())]), users_in_db);
    /// # Ok(())
    /// # }
    ///
    /// # #[cfg(not(feature = "mysql"))]
    /// # fn main() {}
    /// ```
    ///
    /// ## Use `excluded` to get the rejected value
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # #[cfg(any(feature = "sqlite", feature = "postgres"))]
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// use diesel::upsert::excluded;
    ///
    /// #     let conn = &mut establish_connection();
    /// #     #[cfg(feature = "postgres")]
    /// #     diesel::sql_query("TRUNCATE TABLE users").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Pascal" };
    /// let user2 = User { id: 1, name: "Sean" };
    /// let user3 = User { id: 2, name: "Tess" };
    ///
    /// # #[cfg(feature = "postgres")]
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// #[cfg(feature = "postgres")]
    /// let insert_count = diesel::insert_into(users)
    ///     .values(&vec![user2, user3])
    ///     .on_conflict(id)
    ///     .do_update()
    ///     .set(name.eq(excluded(name)))
    ///     .execute(conn);
    /// # #[cfg(feature = "postgres")]
    /// assert_eq!(Ok(2), insert_count);
    ///
    /// # #[cfg(feature = "postgres")]
    /// let users_in_db = users.load(conn);
    /// # #[cfg(feature = "postgres")]
    /// assert_eq!(Ok(vec![(1, "Sean".to_string()), (2, "Tess".to_string())]), users_in_db);
    /// # }
    /// # #[cfg(feature = "mysql")]
    /// # fn main() {}
    /// ```
    ///
    /// ## Use `.filter()`method to limit the rows actually updated
    ///
    /// ```rust
    /// # include!("on_conflict_docs_setup.rs");
    /// #
    /// # #[cfg(feature = "postgres")]
    /// # fn main() {
    /// #     use diesel::QueryDsl;
    /// #     use diesel::query_dsl::methods::FilterDsl;
    /// use self::users::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     #[cfg(feature = "postgres")]
    /// #     diesel::sql_query("TRUNCATE TABLE users").execute(conn).unwrap();
    /// let user = User { id: 1, name: "Pascal" };
    /// let user2 = User { id: 1, name: "Sean" };
    ///
    /// assert_eq!(Ok(1), diesel::insert_into(users).values(&user).execute(conn));
    ///
    /// let insert_count = diesel::insert_into(users)
    ///     .values(&user2)
    ///     .on_conflict(id)
    ///     .do_update()
    ///     .set(&user2)
    ///     .filter(id.ge(5))
    ///     .execute(conn);
    /// assert_eq!(Ok(0), insert_count);
    ///
    /// let users_in_db = users.load(conn);
    /// assert_eq!(Ok(vec![(1, "Pascal".to_string())]), users_in_db);
    /// # }
    /// # #[cfg(any(feature = "sqlite", feature = "mysql"))]
    /// # fn main() {}
    /// ```
    pub fn do_update(self) -> IncompleteDoUpdate<Stmt, Target> {
        IncompleteDoUpdate {
            stmt: self.stmt,
            target: self.target,
        }
    }
}

/// A partially constructed `ON CONFLICT DO UPDATE` clause.
#[derive(Debug, Clone, Copy)]
pub struct IncompleteDoUpdate<Stmt, Target> {
    stmt: Stmt,
    target: Target,
}

impl<T: QuerySource, U, Op, Ret, Target>
    IncompleteDoUpdate<InsertStatement<T, U, Op, Ret>, Target>
{
    /// See [`do_update`] for usage examples.
    ///
    /// [`do_update`]: IncompleteOnConflict::do_update()
    pub fn set<Changes>(
        self,
        changes: Changes,
    ) -> InsertStatement<T, OnConflictValues<U, Target, DoUpdate<Changes::Changeset, T>>, Op, Ret>
    where
        T: QuerySource,
        Changes: AsChangeset<Target = T>,
    {
        let target = self.target;
        self.stmt.replace_values(|values| {
            OnConflictValues::new(
                values,
                target,
                DoUpdate::new(changes.as_changeset()),
                NoWhereClause,
            )
        })
    }
}

impl<T, U, Op, Ret, Target, Action, WhereClause, Predicate> FilterDsl<Predicate>
    for InsertStatement<T, OnConflictValues<U, Target, Action, WhereClause>, Op, Ret>
where
    T: QuerySource,
    WhereClause: WhereAnd<Predicate>,
{
    type Output =
        InsertStatement<T, OnConflictValues<U, Target, Action, WhereClause::Output>, Op, Ret>;

    fn filter(self, predicate: Predicate) -> Self::Output {
        self.replace_values(|values| {
            values.replace_where(|where_clause| where_clause.and(predicate))
        })
    }
}

impl<T, U, Op, Ret, Target, Action, WhereClause, Predicate> OrFilterDsl<Predicate>
    for InsertStatement<T, OnConflictValues<U, Target, Action, WhereClause>, Op, Ret>
where
    T: QuerySource,
    WhereClause: WhereOr<Predicate>,
{
    type Output =
        InsertStatement<T, OnConflictValues<U, Target, Action, WhereClause::Output>, Op, Ret>;

    fn or_filter(self, predicate: Predicate) -> Self::Output {
        self.replace_values(|values| {
            values.replace_where(|where_clause| where_clause.or(predicate))
        })
    }
}
