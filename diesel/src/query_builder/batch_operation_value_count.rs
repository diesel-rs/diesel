use crate::Table;
use crate::associations::Identifiable;
use crate::expression::Expression;
use crate::insertable::Insertable;
use crate::query_builder::{AsChangeset, ValuesClause};
use crate::util::TupleSize;
use core::marker::PhantomData;

/// The number of values each record sends in a batch insert or a batch update
///
/// Use it to split a large batch into statements that stay under the bind
/// parameter limit of the database (65535 on PostgreSQL).
///
/// Every record of a batch implements it, for the operation it takes part in:
///
/// * [`InsertOperation<table>`](InsertOperation): one value per inserted
///   column, for an [`Insertable`] record
/// * [`UpdateOperation`]: one value per primary key column, plus one per
///   updated column, for an [`AsChangeset`] and [`Identifiable`] record
///
/// The operation is inferred from the record type, unless the type is both:
/// then name it, as in `batches::<UpdateOperation, _>(&records, max_binds)`.
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::users;
/// use diesel::query_builder::BatchOperationValueCount;
///
/// /// Splits `records` into batches that send at most `max_binds` bind
/// /// parameters each
/// fn batches<Op, R>(records: &[R], max_binds: usize) -> std::slice::Chunks<'_, R>
/// where
///     R: BatchOperationValueCount<Op>,
/// {
///     records.chunks(max_binds / R::VALUES_PER_RECORD)
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
/// for batch in batches(&new_users, max_binds) {
///     diesel::insert_into(users::table).values(batch).execute(connection)?;
/// }
///
/// // Two values per user, its id and its name: one user per statement
/// let renamed = [User { id: 1, name: "Sam" }, User { id: 2, name: "Tia" }];
/// for batch in batches(&renamed, max_binds) {
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
/// * The operation is a type parameter: two blanket impls over the record
///   without it (`Insertable` and `AsChangeset`) would overlap for a type
///   deriving both, with different counts.
/// * The update impl requires [`Identifiable`]: tuples of `eq` assignments are
///   [`AsChangeset`] too, and would otherwise make the operation ambiguous for
///   the tuples of a batch insert.
/// * Both impls bound the owned record: a bound on `&'a U: AsChangeset` makes
///   rustc evaluate `&?U: AsChangeset` before `U` is inferred, which overflows
///   on the recursive impls for references.
/// * The counts come from `TupleSize`, implemented for the insert column values
///   (`ColumnInsertValue`, `DefaultableColumnInsertValue`) and for `Assign`: a
///   new kind of column value or assignment needs it too.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a record of a batch insert or a batch update",
    note = "a batch insert takes `Insertable` records whose values are a fixed set of columns, \
            a batch update `AsChangeset` and `Identifiable` records whose changeset assigns a fixed set of columns"
)]
pub trait BatchOperationValueCount<Operation> {
    /// The number of values each record sends in that operation
    const VALUES_PER_RECORD: usize;
}

/// A batch insert into `Tab`, as the operation of a [`BatchOperationValueCount`]
#[derive(Debug, Clone, Copy)]
pub struct InsertOperation<Tab>(PhantomData<Tab>);

/// A batch update, as the operation of a [`BatchOperationValueCount`]
#[derive(Debug, Clone, Copy)]
pub struct UpdateOperation(());

impl<E, V, Tab> BatchOperationValueCount<InsertOperation<Tab>> for E
where
    E: Insertable<Tab, Values = ValuesClause<V, Tab>>,
    V: TupleSize,
{
    const VALUES_PER_RECORD: usize = V::SIZE;
}

impl<U> BatchOperationValueCount<UpdateOperation> for U
where
    U: AsChangeset<Changeset: TupleSize>,
    for<'a> &'a U: Identifiable,
    U::Target: Table<PrimaryKey: Expression<SqlType: TupleSize>>,
{
    const VALUES_PER_RECORD: usize =
        <<<U::Target as Table>::PrimaryKey as Expression>::SqlType as TupleSize>::SIZE
            + <U::Changeset as TupleSize>::SIZE;
}
