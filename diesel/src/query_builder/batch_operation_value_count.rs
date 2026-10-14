use crate::Table;
use crate::expression::Expression;
use crate::insertable::Insertable;
use crate::query_builder::{AsChangeset, IncompleteInsertStatement, UpdateStatement, ValuesClause};
use crate::util::TupleSize;

/// The number of values each record sends in a batch insert or a batch update
///
/// Use it to split a large batch into statements that stay under the bind
/// parameter limit of the database (65535 on PostgreSQL).
///
/// [`insert_into`](crate::insert_into()) and [`update`](crate::update())
/// implement it for the records of a batch:
///
/// * a batch insert sends one value per inserted column
/// * a batch update sends one value per primary key column, plus one per
///   updated column
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::users;
/// use diesel::query_builder::BatchOperationValueCount;
///
/// /// Splits `records` into batches that send at most `max_binds` bind
/// /// parameters through `statement`
/// fn batches<'a, S, R>(statement: &S, records: &'a [R], max_binds: usize) -> std::slice::Chunks<'a, R>
/// where
///     S: BatchOperationValueCount<R>,
/// {
///     records.chunks(max_binds / statement.values_per_record(records))
/// }
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
/// // 65535 on PostgreSQL, 3 here to show the split
/// let max_binds = 3;
///
/// // One value per new user, its name: both in one statement
/// let new_users = [NewUser { name: "Jim" }, NewUser { name: "Tom" }];
/// for batch in batches(&diesel::insert_into(users::table), &new_users, max_binds) {
///     diesel::insert_into(users::table).values(batch).execute(connection)?;
/// }
///
/// // Two values per user, its id and its name: one user per statement
/// let renamed = [User { id: 1, name: "Sam" }, User { id: 2, name: "Tia" }];
/// for batch in batches(&diesel::update(users::table), &renamed, max_binds) {
///     diesel::update(users::table).set(batch).execute(connection)?;
/// }
/// #
/// #     let names = users::table
/// #         .select(users::name)
/// #         .order(users::id)
/// #         .load::<String>(connection)?;
/// #     assert_eq!(vec!["Sam", "Tia", "Jim", "Tom"], names);
/// #     Ok(())
/// # }
/// ```
///
/// # Values and bind parameters
///
/// A value is one bind parameter when the field holds a Rust value. The count
/// is an upper bound when a field sends none ([`now`](crate::dsl::now), or a
/// `None` inserted as `DEFAULT`), and too low when a field holds an expression
/// with several (`a.concat(b)`). The bind parameters of the other clauses of
/// the statement, such as `WHERE`, are not counted.
///
/// # For Maintainers
///
/// * The statements implement this trait rather than the records: a type
///   deriving both `Insertable` and `AsChangeset` would otherwise get two
///   overlapping blanket impls, with different counts.
/// * Both impls bound the owned record: a bound on `&'a U: AsChangeset` makes
///   rustc evaluate `&?U: AsChangeset` before `U` is inferred, which overflows
///   on the recursive impls for references.
/// * The counts come from `TupleSize`, implemented for the insert column values
///   (`ColumnInsertValue`, `DefaultableColumnInsertValue`) and for `Assign`: a
///   new kind of column value or assignment needs it too.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot count the values a `{Record}` sends in a batch",
    note = "a batch insert counts an `Insertable` type whose values are a fixed set of columns, \
            a batch update an `AsChangeset` type whose changeset assigns a fixed set of columns of a table with a primary key"
)]
pub trait BatchOperationValueCount<Record> {
    /// The number of values each record sends through this statement
    const VALUES_PER_RECORD: usize;

    /// [`VALUES_PER_RECORD`](Self::VALUES_PER_RECORD), for the record type of
    /// `records`: call it on `insert_into(table)` or `update(table)` without
    /// writing their types
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
