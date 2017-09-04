use expression::{AppearsOnTable, Expression};
use pg::Pg;
use query_builder::*;
use query_source::*;
use result::QueryResult;

/// Used in conjunction with
/// [`on_conflict`](trait.OnConflictExtension.html#method.on_conflict) to write
/// a query in the form `ON CONFLICT (name) DO NOTHING`. If you want to do
/// nothing when *any* constraint conflicts, use
/// [`on_conflict_do_nothing()`](trait.OnConflictExtension.html#method.on_conflict_do_nothing)
/// instead.
pub fn do_nothing() -> DoNothing {
    DoNothing
}

/// Used to create a query in the form `ON CONFLICT (...) DO UPDATE ...`
///
/// Call `.set` on the result of this function with the changes you want to
/// apply. The argument to `set` can be anything that implements `AsChangeset`
/// (e.g. anything you could pass to `set` on a normal update statement).
///
/// Note: When inserting more than one row at a time, this query can still fail
/// if the rows being inserted conflict with each other.
///
/// # Examples
///
/// ## Set specific value on conflict
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_codegen;
/// # include!("on_conflict_docs_setup.rs");
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// use self::diesel::pg::upsert::*;
///
/// #     let conn = establish_connection();
/// #     conn.execute("TRUNCATE TABLE users").unwrap();
/// let user = User { id: 1, name: "Pascal" };
/// let user2 = User { id: 1, name: "Sean" };
///
/// assert_eq!(Ok(1), diesel::insert(&user).into(users).execute(&conn));
///
/// let insert_count = diesel::insert(
///     &user2.on_conflict(id, do_update().set(name.eq("I DONT KNOW ANYMORE")))
/// ).into(users).execute(&conn);
/// assert_eq!(Ok(1), insert_count);
///
/// let users_in_db = users.load(&conn);
/// assert_eq!(Ok(vec![(1, "I DONT KNOW ANYMORE".to_string())]), users_in_db);
/// # }
/// ```
///
/// ## Set `AsChangeset` struct on conflict
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_codegen;
/// # include!("on_conflict_docs_setup.rs");
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// use self::diesel::pg::upsert::*;
///
/// #     let conn = establish_connection();
/// #     conn.execute("TRUNCATE TABLE users").unwrap();
/// let user = User { id: 1, name: "Pascal" };
/// let user2 = User { id: 1, name: "Sean" };
///
/// assert_eq!(Ok(1), diesel::insert(&user).into(users).execute(&conn));
///
/// let insert_count = diesel::insert(
///     &user2.on_conflict(id, do_update().set(&user2))
/// ).into(users).execute(&conn);
/// assert_eq!(Ok(1), insert_count);
///
/// let users_in_db = users.load(&conn);
/// assert_eq!(Ok(vec![(1, "Sean".to_string())]), users_in_db);
/// # }
/// ```
///
/// ## Use `excluded` to get the rejected value
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_codegen;
/// # include!("on_conflict_docs_setup.rs");
/// #
/// # fn main() {
/// #     use self::users::dsl::*;
/// use self::diesel::pg::upsert::*;
///
/// #     let conn = establish_connection();
/// #     conn.execute("TRUNCATE TABLE users").unwrap();
/// let user = User { id: 1, name: "Pascal" };
/// let user2 = User { id: 1, name: "Sean" };
/// let user3 = User { id: 2, name: "Tess" };
///
/// assert_eq!(Ok(1), diesel::insert(&user).into(users).execute(&conn));
///
/// let insert_count = diesel::insert(&vec![user2, user3]
///     .on_conflict(id, do_update().set(name.eq(excluded(name))))
/// ).into(users).execute(&conn);
/// assert_eq!(Ok(2), insert_count);
///
/// let users_in_db = users.load(&conn);
/// assert_eq!(Ok(vec![(1, "Sean".to_string()), (2, "Tess".to_string())]), users_in_db);
/// # }
pub fn do_update() -> IncompleteDoUpdate {
    IncompleteDoUpdate
}

/// Represents `excluded.column` in an `ON CONFLICT DO UPDATE` clause.
pub fn excluded<T>(excluded: T) -> Excluded<T> {
    Excluded(excluded)
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct DoNothing;

impl QueryFragment<Pg> for DoNothing {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql(" DO NOTHING");
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IncompleteDoUpdate;

impl IncompleteDoUpdate {
    pub fn set<T: AsChangeset>(self, changeset: T) -> DoUpdate<T> {
        DoUpdate {
            changeset: changeset,
        }
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct DoUpdate<T> {
    changeset: T,
}

impl<T> QueryFragment<Pg> for DoUpdate<T> where
    T: Changeset<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        if self.changeset.is_noop() {
            out.push_sql(" DO NOTHING");
        } else {
            out.push_sql(" DO UPDATE SET ");
            self.changeset.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct Excluded<T>(T);

impl<T> QueryFragment<Pg> for Excluded<T> where
    T: Column,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("excluded.");
        try!(out.push_identifier(T::NAME));
        Ok(())
    }
}

impl<T> Expression for Excluded<T> where
    T: Expression,
{
    type SqlType = T::SqlType;
}

impl<T> AppearsOnTable<T::Table> for Excluded<T> where
    T: Column,
    Excluded<T>: Expression,
{
}

#[doc(hidden)]
pub trait IntoConflictAction<T> {
    type Action: QueryFragment<Pg>;

    fn into_conflict_action(self) -> Self::Action;
}

impl<T> IntoConflictAction<T> for DoNothing {
    type Action = Self;

    fn into_conflict_action(self) -> Self::Action {
        self
    }
}

impl<Table, Changes> IntoConflictAction<Table> for DoUpdate<Changes> where
    Table: QuerySource,
    Changes: AsChangeset<Target=Table>,
    DoUpdate<Changes::Changeset>: QueryFragment<Pg>,
{
    type Action = DoUpdate<Changes::Changeset>;

    fn into_conflict_action(self) -> Self::Action {
        DoUpdate {
            changeset: self.changeset.as_changeset()
        }
    }
}
