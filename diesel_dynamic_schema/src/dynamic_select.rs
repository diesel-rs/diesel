use diesel::backend::Backend;
use diesel::expression::{is_aggregate, NonAggregate, ValidGrouping};
use diesel::query_builder::{AstPass, QueryFragment, QueryId};
use diesel::sql_types::Untyped;
use diesel::{AppearsOnTable, Expression, QueryResult, SelectableExpression};
use std::iter::FromIterator;
use std::marker::PhantomData;

/// Represents a dynamically sized select clause
#[allow(missing_debug_implementations)]
pub struct DynamicSelectClause<'a, DB, QS> {
    selects: Vec<Box<dyn QueryFragment<DB> + Send + 'a>>,
    p: PhantomData<QS>,
}

impl<DB, QS> QueryId for DynamicSelectClause<'_, DB, QS> {
    const HAS_STATIC_QUERY_ID: bool = false;
    type QueryId = ();
}

impl<DB, QS> Default for DynamicSelectClause<'_, DB, QS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, DB, QS> DynamicSelectClause<'a, DB, QS> {
    /// Constructs a new dynamically sized select clause without any fields
    pub fn new() -> Self {
        Self {
            selects: Vec::new(),
            p: PhantomData,
        }
    }

    /// Adds the field to the dynamically sized select clause
    pub fn add_field<F>(&mut self, field: F)
    where
        F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
        DB: Backend,
    {
        self.selects.push(Box::new(field))
    }

    /// Boxes a field as a type-erased `QueryFragment`, using this select clause's
    /// own `DB`/`QS` type parameters to resolve them for the conversion.
    ///
    /// This is a thin wrapper around [`IntoBoxedField::into_boxed`] that exists
    /// purely for type inference: `QS` does not appear anywhere in the boxed
    /// return type, so calling `field.into_boxed()` directly on a loose
    /// expression usually leaves the compiler unable to infer it. Going through
    /// an existing `&DynamicSelectClause<'a, DB, QS>` pins both `DB` and `QS`
    /// from `self`'s own type, so no annotation or turbofish is needed.
    pub fn box_field<F>(&self, field: F) -> Box<dyn QueryFragment<DB> + Send + 'a>
    where
        F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
        DB: Backend,
    {
        Box::new(field)
    }

    /// Add multiple already boxed fields to the dynamically sized select clause
    /// in a single call.
    ///
    /// Unlike [`add_fields`](Self::add_fields), the fields do not all need to share
    /// the same Rust type, since each one has already been type-erased via
    /// [`box_field`](Self::box_field) (or [`IntoBoxedField::into_boxed`] directly).
    /// This is useful when the set of columns (and their SQL types) is only
    /// known at runtime, e.g. when it was discovered by introspecting the
    /// database schema.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../tests/connection_setup.rs");
    /// # use diesel::prelude::*;
    /// # use diesel::sql_types::{Integer, Text};
    /// # use diesel_dynamic_schema::{table, DynamicSelectClause, Table};
    /// #
    /// # #[cfg(feature = "sqlite")]
    /// # fn main() {
    /// # let conn = &mut establish_connection();
    /// # create_user_table(conn);
    /// let users = table("users");
    /// let id = users.column::<Integer, _>("id");
    /// let name = users.column::<Text, _>("name");
    ///
    /// // The select clause's own `DB`/`QS` need to be known before boxing
    /// // anything into it; here that's the same pair you'd normally get for
    /// // free from a later `.select(select)` call. Annotate it up front if you
    /// // want to build the boxed `Vec` before wiring up the rest of the query.
    /// let mut select: DynamicSelectClause<diesel::sqlite::Sqlite, Table<&str>> =
    ///     DynamicSelectClause::new();
    ///
    /// // `id` and `name` have different Rust types (`Column<_, _, Integer>` vs.
    /// // `Column<_, _, Text>`), so they can't be collected into a single `Vec`
    /// // without boxing them first. `box_field` uses `select`'s own type to
    /// // resolve the boxing, so no turbofish is required.
    /// let boxed_fields = vec![select.box_field(id), select.box_field(name)];
    ///
    /// select.add_boxed_fields(boxed_fields);
    /// assert_eq!(select.len(), 2);
    /// # }
    /// # #[cfg(not(feature = "sqlite"))]
    /// # fn main() {}
    /// ```
    pub fn add_boxed_fields<I>(&mut self, fields: I)
    where
        I: IntoIterator<Item = Box<dyn QueryFragment<DB> + Send + 'a>>,
    {
        self.selects.extend(fields);
    }

    /// Add multiple fields to the dynamically sized select clause
    pub fn add_fields<I, F>(&mut self, fields: I)
    where
        I: IntoIterator<Item = F>,
        F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
        DB: Backend,
    {
        for field in fields {
            self.add_field(field);
        }
    }

    /// Returns the number of fields in the select clause
    pub fn len(&self) -> usize {
        self.selects.len()
    }

    /// Returns whether the select clause is empty
    pub fn is_empty(&self) -> bool {
        self.selects.is_empty()
    }
}

impl<DB, QS> AppearsOnTable<QS> for DynamicSelectClause<'_, DB, QS> where Self: Expression {}

impl<DB, QS> SelectableExpression<QS> for DynamicSelectClause<'_, DB, QS> where
    Self: AppearsOnTable<QS>
{
}

impl<QS, DB> Expression for DynamicSelectClause<'_, DB, QS> {
    type SqlType = Untyped;
}

impl<DB, QS> QueryFragment<DB> for DynamicSelectClause<'_, DB, QS>
where
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        let mut first = true;
        for s in &self.selects {
            if first {
                first = false;
            } else {
                pass.push_sql(", ");
            }
            s.walk_ast(pass.reborrow())?;
        }
        Ok(())
    }
}

impl<DB, QS> ValidGrouping<()> for DynamicSelectClause<'_, DB, QS> {
    type IsAggregate = is_aggregate::No;
}

impl<'a, DB, QS, F> FromIterator<F> for DynamicSelectClause<'a, DB, QS>
where
    F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
    DB: Backend,
{
    fn from_iter<I: IntoIterator<Item = F>>(iter: I) -> Self {
        let mut select_clause = DynamicSelectClause::new();
        select_clause.add_fields(iter);
        select_clause
    }
}

impl<'a, DB, QS, F> std::iter::Extend<F> for DynamicSelectClause<'a, DB, QS>
where
    F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
    DB: Backend,
{
    fn extend<I: IntoIterator<Item = F>>(&mut self, iter: I) {
        self.add_fields(iter)
    }
}

/// Type-erases a column/expression so it can be stored alongside other
/// fields of different SQL types, e.g. via [`DynamicSelectClause::add_boxed_fields`].
pub trait IntoBoxedField<'a, DB, QS> {
    /// Boxes `self` as a type-erased `QueryFragment`.
    ///
    /// This drops the field's concrete Rust type (including its SQL type),
    /// while preserving its ability to be walked into a SQL query.
    fn into_boxed(self) -> Box<dyn QueryFragment<DB> + Send + 'a>;
}

impl<'a, DB, QS, F> IntoBoxedField<'a, DB, QS> for F
where
    F: QueryFragment<DB> + SelectableExpression<QS> + NonAggregate + Send + 'a,
    DB: Backend,
{
    fn into_boxed(self) -> Box<dyn QueryFragment<DB> + Send + 'a> {
        Box::new(self)
    }
}
