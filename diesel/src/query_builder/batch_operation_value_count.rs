use crate::Table;
use crate::expression::Expression;
use crate::insertable::Insertable;
use crate::query_builder::{AsChangeset, IncompleteInsertStatement, UpdateStatement, ValuesClause};
use crate::util::TupleSize;

/// The number of values each record sends in a batch insert or a batch update
///
/// A batch statement sends values for each of its records: a batch insert one
/// for each inserted column, a batch update one for each column of the primary
/// key plus one for each updated column. Splitting a large batch into
/// statements that stay under the bind parameter limit of the database (65535
/// on PostgreSQL) needs that number.
///
/// The statement a batch is sent through implements this trait for the type of
/// its records:
///
/// * [`IncompleteInsertStatement`], returned by
///   [`insert_into`](crate::insert_into()), for every [`Insertable`] type whose
///   values are a fixed set of columns, which includes the types deriving
///   [`Insertable`](derive@crate::Insertable) and tuples of
///   [`eq`](crate::ExpressionMethods::eq) assignments
/// * [`UpdateStatement`], returned by [`update`](crate::update()), for every
///   [`AsChangeset`] type whose changeset assigns a fixed set of columns of a
///   table with a primary key, which includes the records of a
///   [batch update](crate::update()#batch-update)
///
/// The statement rather than the record implements it because a type can be
/// both inserted and updated, with a different count for each.
///
/// # Values and bind parameters
///
/// Each value is a bind parameter when the record holds a Rust value for that
/// column. An SQL expression sends its own bind parameters instead: none for
/// [`now`](crate::dsl::now), two for `a.concat(b)`, and a `None` field of a
/// batch insert sends `DEFAULT`, which has none. The count is therefore the
/// number of bind parameters of a record when all its fields hold Rust values,
/// an upper bound of it when some fields send no bind parameter, and less than
/// it when some hold an expression with several.
///
/// It counts the values of the records only: the other clauses of the
/// statement, such as a `WHERE` clause or the `SET` clause of an
/// `ON CONFLICT DO UPDATE`, add their bind parameters once per statement. On
/// SQLite, a batch insert of records that hold an `Option` field already runs
/// one statement per record.
///
/// # Example
///
/// Splitting batches into statements that stay under a bind parameter limit:
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::users;
/// use diesel::query_builder::BatchOperationValueCount;
///
/// #[derive(Insertable)]
/// #[diesel(table_name = users)]
/// struct NewUser<'a> {
///     name: &'a str,
/// }
///
/// #[derive(AsChangeset, Identifiable)]
/// struct User<'a> {
///     id: i32,
///     name: &'a str,
/// }
///
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// // The bind parameter limit of the database
/// let max_binds = 3;
///
/// let insert = diesel::insert_into(users::table);
/// let new_users = [NewUser { name: "Jim" }, NewUser { name: "Tom" }];
/// // Each new user sends its name: both in one statement
/// assert_eq!(1, insert.values_per_record(&new_users));
/// for batch in new_users.chunks(max_binds / insert.values_per_record(&new_users)) {
///     insert.values(batch).execute(connection)?;
/// }
///
/// let renamed = [User { id: 1, name: "Sam" }, User { id: 2, name: "Tia" }];
/// // Each user sends its id and its name: one user per statement
/// let per_record = diesel::update(users::table).values_per_record(&renamed);
/// assert_eq!(2, per_record);
/// for batch in renamed.chunks(max_binds / per_record) {
///     diesel::update(users::table).set(batch).execute(connection)?;
/// }
///
/// let names = users::table
///     .select(users::name)
///     .order(users::id)
///     .load::<String>(connection)?;
/// assert_eq!(vec!["Sam", "Tia", "Jim", "Tom"], names);
/// #     Ok(())
/// # }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot count the values a `{Record}` sends in a batch",
    note = "a batch insert counts an `Insertable` type whose values are a fixed set of columns, \
            a batch update an `AsChangeset` type whose changeset assigns a fixed set of columns of a table with a primary key"
)]
pub trait BatchOperationValueCount<Record> {
    /// The number of values each record sends in a batch sent through this
    /// statement
    ///
    /// See the [trait documentation](BatchOperationValueCount) for how it
    /// relates to bind parameters.
    const VALUES_PER_RECORD: usize;

    /// [`VALUES_PER_RECORD`](Self::VALUES_PER_RECORD) for the type of
    /// `records`
    ///
    /// `records` is read for its type only, so that this statement's count for
    /// it is found without naming the statement's type.
    fn values_per_record(&self, _records: &[Record]) -> usize {
        Self::VALUES_PER_RECORD
    }
}

impl<E, V, Tab, Op> BatchOperationValueCount<E> for IncompleteInsertStatement<Tab, Op>
where
    E: Insertable<Tab, Values = ValuesClause<V, Tab>>,
    V: TupleSize,
{
    const VALUES_PER_RECORD: usize = V::SIZE;
}

impl<U, Tab, W> BatchOperationValueCount<U> for UpdateStatement<Tab, W>
where
    U: AsChangeset<Target = Tab, Changeset: TupleSize>,
    Tab: Table<PrimaryKey: Expression<SqlType: TupleSize>>,
{
    const VALUES_PER_RECORD: usize = <<Tab::PrimaryKey as Expression>::SqlType as TupleSize>::SIZE
        + <U::Changeset as TupleSize>::SIZE;
}
