use super::batch_update::*;
use crate::backend::DieselReserveSpecialization;
use crate::expression::grouped::Grouped;
use crate::expression::operators::Eq;
use crate::expression::AppearsOnTable;
use crate::query_builder::*;
use crate::query_source::{Column, QuerySource};
use crate::Table;
use std::marker::PhantomData;

/// Types which can be passed to
/// [`update.set`](UpdateStatement::set()).
///
/// This trait can be [derived](derive@AsChangeset)
pub trait AsChangeset {
    /// The table which `Self::Changeset` will be updating
    type Target: QuerySource;

    /// The update statement this type represents
    type Changeset;

    /// Convert `self` into the actual update statement being executed
    // This method is part of our public API
    // we won't change it to just appease clippy
    #[allow(clippy::wrong_self_convention)]
    fn as_changeset(self) -> Self::Changeset;
}

// This is a false positive, we reexport it later
#[allow(unreachable_pub)]
#[doc(inline)]
pub use diesel_derives::AsChangeset;

impl<T: AsChangeset> AsChangeset for Option<T> {
    type Target = T::Target;
    type Changeset = Option<T::Changeset>;

    fn as_changeset(self) -> Self::Changeset {
        self.map(AsChangeset::as_changeset)
    }
}

impl<'update, T> AsChangeset for &'update Option<T>
where
    &'update T: AsChangeset,
{
    type Target = <&'update T as AsChangeset>::Target;
    type Changeset = Option<<&'update T as AsChangeset>::Changeset>;

    fn as_changeset(self) -> Self::Changeset {
        self.as_ref().map(AsChangeset::as_changeset)
    }
}

impl<Left, Right> AsChangeset for Eq<Left, Right>
where
    Left: AssignmentTarget,
    Right: AppearsOnTable<Left::Table>,
{
    type Target = Left::Table;
    type Changeset = Assign<<Left as AssignmentTarget>::QueryAstNode, Right>;

    fn as_changeset(self) -> Self::Changeset {
        Assign {
            target: self.left.into_target(),
            expr: self.right,
        }
    }
}

impl<Left, Right> AsChangeset for Grouped<Eq<Left, Right>>
where
    Eq<Left, Right>: AsChangeset,
{
    type Target = <Eq<Left, Right> as AsChangeset>::Target;

    type Changeset = <Eq<Left, Right> as AsChangeset>::Changeset;

    fn as_changeset(self) -> Self::Changeset {
        self.0.as_changeset()
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Assign<Target, Expr> {
    target: Target,
    expr: Expr,
}

impl<T, U, DB> QueryFragment<DB> for Assign<T, U>
where
    DB: Backend,
    T: QueryFragment<DB>,
    U: QueryFragment<DB>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&self.target, out.reborrow())?;
        out.push_sql(" = ");
        QueryFragment::walk_ast(&self.expr, out.reborrow())
    }
}

/// Represents the left hand side of an assignment expression for an
/// assignment in [AsChangeset]. The vast majority of the time, this will
/// be a [Column]. However, in certain database backends, it's possible to
/// assign to an expression. For example, in Postgres, it's possible to
/// "UPDATE TABLE SET array_column\[1\] = 'foo'".
pub trait AssignmentTarget {
    /// Table the assignment is to
    type Table: Table;
    /// A wrapper around a type to assign to (this wrapper should implement
    /// [QueryFragment]).
    type QueryAstNode;

    /// Move this in to the AST node which should implement [QueryFragment].
    fn into_target(self) -> Self::QueryAstNode;
}

/// Represents a `Column` as an `AssignmentTarget`. The vast majority of
/// targets in an update statement will be `Column`s.
#[derive(Debug, Clone, Copy)]
pub struct ColumnWrapperForUpdate<C>(pub C);

impl<DB, C> QueryFragment<DB> for ColumnWrapperForUpdate<C>
where
    DB: Backend + DieselReserveSpecialization,
    C: Column,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_identifier(C::NAME)
    }
}

impl<C> AssignmentTarget for C
where
    C: Column,
{
    type Table = C::Table;
    type QueryAstNode = ColumnWrapperForUpdate<C>;

    fn into_target(self) -> Self::QueryAstNode {
        ColumnWrapperForUpdate(self)
    }
}

/*
    Run batch update unit test: (not working obviously)
    cargo test -p diesel_derives -F diesel/postgres -F diesel_derives/postgres named_struct_batch -- --nocapture

    Sources:
    Source_D: DERIVE impl AsChangeset       -> diesel/diesel_derives/src/as_changeset.rs
    Source_T: TUPLE impl for all traits     -> diesel/diesel/src/type_impls/tuples.rs
    Source_M: MACRO for tuple combinations  -> diesel/diesel_derives/src/diesel_for_each_tuple.rs
    Source_U: UPDATE_STATEMENT main logic   -> diesel/diesel/src/query_builder/update_statement/mod.rs
    Source_I: INSERTABLE trait impl         -> diesel/diesel/src/insertable.rs

    --------------------------------------------------------------------------------------------------

    My understanding so far:

    #[derive(AsChangeset)]
    struct Example {
        a: String,
        b: i32,
    }
    let example = Example { a: "my_string".to_string() , b: "61" };

    1.  The derive AsChangeset from Source_D lays out all fields of a struct as a tuple and calls
        AsChangeset::as_changeset on the tuple. The struct fields turn into:
            Eq<column, field_name> or Option<Eq<column, field_name>> respectively and
            Eq<column, field_type> or Option<Eq<column, field_type>> respectively
        For our 'Example' struct:
            Eq<Table::a, example.a> and Eq<Table::a_type, String>
            Eq<Table::b, example.b> and Eq<Table::b_type, i32>
        where column name of a field (e.g. example.a) has to match table column name (e.g. Table::a).
        This results in a call like:
            AsChangeset::as_changeset( (Eq<Table::a, example.a>, Eq<Table::b, example.b>) );
        lets call this Func1 going forward.

    2.  All traits are implemented directly on the tuples (see Source_T). The implementation
        of AsChangeset for a tuple in turn subsequently calls AsChangeset::as_changeset for
        each of its elements. Func1 becomes:
            AsChangeset::as_changeset( Eq<Table::a, example.a> );
            AsChangeset::as_changeset( Eq<Table::b, example.b> );

    3.  AsChangeset::as_changeset recursively calls AsChangeset::as_changeset for all structs
        with matching bounds. Using 'impl<Left, Right> AsChangeset for Eq<Left, Right>' (and
        proper trait bounds) we now can access our fields (see above line 53). A struct that
        implements QueryFragment has to be returned by the end of this recursion. That's the
        part defining the behavior of UPDATE_STATEMENT::values.walk_ast(...) (see Source_U).
        That generates and pushes the following to the end of the current sql statement:
            "'table'.'a' = "my_string", 'table'.'b' = 61"

    --------------------------------------------------------------------------------------------------

    The following is unclear to me and needs clarification please:

    I understand that in Source_M all combinations for the tuples (0...MAX_TUPLE_SIZE)
    use the macro from Source_T 'tuple_impls':
    Counting by the index (please forgive my off by one error, if I made one):
    Tuple with no elements like () ?
        tuple_impls{ 1 { (0) -> T, ST, TT } }
    Tuple with no one element like (one) ?
        tuple_impls{ 2 { (0) -> T, ST, TT, (1) -> T1, ST1, TT1, } }
    Tuple with no two elements like (one, two) ?
        tuple_impls{ 3 { (0) -> T, ST, TT, (1) -> T1, ST1, TT1, (2) -> T2, ST2, TT2, } }
    and so on until MAX_TUPLE_SIZE is reached.

    I could not figure out the meaning of $T:ident and $ST:ident in 'tuple_impls' for each tuple element.
    How does a tuple translate to T, ST and TT ? (maybe TT is unimportant since unused)
    Maybe:
        impl<$($T,)+ Tab> for (Eq<Table::a, example.a>, Eq<Table::b, example.b>)
        T  = type representing the individual elements Eq<Left, Right>

        impl<$($T,)+ $($ST,)+ Tab> for (Eq<Table::a, example.a>, Eq<Table::b, example.b>)
        T  = type of Left for all Eq<Left, Right>
        ST = type of Right for all Eq<Left, Right>
    Or:
        T  = always the type for Eq<Table::a, example.a>
        ST = stands for Bound<Text, &String> since example.a is of type String

    I'm just guessing at this point and would appreciate some explanation...

    --------------------------------------------------------------------------------------------------

    TODO:
    1.  Implement trait UpdateValues similar to InsertValues (see Source_I).
        That should handle convenient sequential listing of columns.

    2.  Figure out how the values can be accumulated similar to Insertable (see Source_I).
            impl<$($T,)+ $($ST,)+ Tab> Insertable<Tab> for ($($T,)+) from Source_T seems promising.

    3.  Create an enum wrapper to store the result of AsChangeset::as_changeset recursion. E.g.
            enum UpdateValuesMethod<T> { Assign<T>, Batch<T>, }

    4.  Create and implement a new trait for the enum UpdateValuesMethod such that it can
        explicitely be bound by it.
            trait UpdateValuesMethodTrait<T> {}
            impl<T> UpdateValuesMethodTrait<T> for UpdateValuesMethod<T> {}

    5.  Add UpdateValuesMethodTrait trait to the bounds for UpdateStatement::values
        impl<T, U, V, Ret, DB> QueryFragment<DB> for UpdateStatement<T, U, V, Ret>
        ...
            V: QueryFragment<DB> + UpdateValuesMethodTrait<DB>

    6.  Specialize the implementation for UpdateStatement::walk_ast (see Source_U) to cover
        for the different cases of the enum UpdateValuesMethod.

        UpdateValuesMethod::Assign:
            Should remain unchanges from its current statement.

        UpdateValuesMethod::Batch:
            Postgress example for batch update similar to
            UPDATE .. SET .. FROM ( VALUES (..), (..) ) .. WHERE .. ;

            UPDATE my_table AS tab SET
                column_a = tmp.column_a,
                column_c = tmp.column_c
            FROM ( VALUES
                ('aa', 1, 11),
                ('bb', 2, 22)
            ) AS tmp(column_a, column_b, column_c)
            WHERE tab.column_b = tmp.column_b;

            Note that calling filter() on an update statement would corrupt the query.
            Maybe it makes sense to ignore the UpdateStatement::where_clause for Batches?

    7.  Add constraint for UpdateValuesMethod::Batch such that tab.column_b should
        not appear in the SET clause if it was specified in the WHERE clause.

    8.  Add a mechanism to conveniently specify which column should be used in the WHERE clause.

    9.  Look into batch updates for MySql and SQLite and apply them.

    10. Pray: Hope I can nail it!

    Lastly.
        apply diesel practices
        write tests
        write documentation
*/

// Copied from diesel/diesel/src/insertable.rs

impl<'a, T> AsChangeset for &'a [T]
where
    T: AsChangeset,
    // TODO: [UndecoratedUpdateRecord] could be used to enforce setting a where_clause maybe.
    // &'a T: AsChangeset + UndecoratedUpdateRecord<T::Target>,
    &'a T: AsChangeset,
{
    type Target = T::Target;
    type Changeset = BatchChangeSet<<&'a T as AsChangeset>::Changeset, T::Target>;

    fn as_changeset(self) -> Self::Changeset {
        let values = self
            .iter()
            .map(AsChangeset::as_changeset)
            .collect::<Vec<_>>();
        BatchChangeSet {
            values: values,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> AsChangeset for &'a Vec<T>
where
    T: AsChangeset,
    &'a [T]: AsChangeset,
{
    type Target = T::Target;
    type Changeset = <&'a [T] as AsChangeset>::Changeset;

    fn as_changeset(self) -> Self::Changeset {
        (&**self).as_changeset()
    }
}

impl<T> AsChangeset for Vec<T>
where
    T: AsChangeset + UndecoratedUpdateRecord<T::Target>,
{
    type Target = T::Target;

    type Changeset = BatchUpdate<Vec<T::Changeset>, T::Target, (), false>;

    fn as_changeset(self) -> Self::Changeset {
        let values = self
            .into_iter()
            .map(AsChangeset::as_changeset)
            .collect::<Vec<_>>();

        BatchUpdate::new(values)
    }
}

impl<T, const N: usize> AsChangeset for [T; N]
where
    T: AsChangeset,
{
    type Target = T::Target;
    type Changeset = BatchUpdate<Vec<T::Changeset>, T::Target, [T::Changeset; N], true>;

    fn as_changeset(self) -> Self::Changeset {
        let values = self
            .into_iter()
            .map(AsChangeset::as_changeset)
            .collect::<Vec<_>>();
        BatchUpdate::new(values)
    }
}

impl<'a, T, const N: usize> AsChangeset for &'a [T; N]
where
    T: AsChangeset,
    &'a T: AsChangeset,
{
    // We can reuse the query id for [T; N] here as this
    // compiles down to the same query
    type Target = T::Target;
    type Changeset =
        BatchUpdate<Vec<<&'a T as AsChangeset>::Changeset>, T::Target, [T::Changeset; N], true>;

    fn as_changeset(self) -> Self::Changeset {
        let values = self.iter().map(AsChangeset::as_changeset).collect();
        BatchUpdate::new(values)
    }
}

impl<T, const N: usize> AsChangeset for Box<[T; N]>
where
    T: AsChangeset,
{
    // We can reuse the query id for [T; N] here as this
    // compiles down to the same query
    type Target = T::Target;
    type Changeset = BatchUpdate<Vec<T::Changeset>, T::Target, [T::Changeset; N], true>;

    fn as_changeset(self) -> Self::Changeset {
        let v = Vec::from(self as Box<[T]>);
        let values = v
            .into_iter()
            .map(AsChangeset::as_changeset)
            .collect::<Vec<_>>();
        BatchUpdate::new(values)
    }
}

/// Marker trait to indicate that no additional operations have been added
/// to a record for update.
///
/// This is used to prevent things like
/// `.on_conflict_do_nothing().on_conflict_do_nothing()`
/// from compiling.
#[cfg_attr(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")
)]
pub trait UndecoratedUpdateRecord<Table> {}

impl<Col, U, DB> BatchUpdateTarget<DB, Col::Table> for Assign<ColumnWrapperForUpdate<Col>, U>
where
    DB: Backend,
    Col: Column,
    ColumnWrapperForUpdate<Col>: QueryFragment<DB>,
    Self: QueryFragment<DB>,
{
    fn column_target<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&self.target, out.reborrow())
    }
}

impl<Col, U, DB> BatchUpdateTargetAssign<DB, Col::Table> for Assign<ColumnWrapperForUpdate<Col>, U>
where
    DB: Backend,
    Col: Column,
    ColumnWrapperForUpdate<Col>: QueryFragment<DB>,
    Self: QueryFragment<DB>,
{
    fn column_target_assign<'b>(
        &'b self,
        mut out: AstPass<'_, 'b, DB>,
        alias: &str,
    ) -> QueryResult<()> {
        let _ = BatchUpdateTarget::column_target(self, out.reborrow());
        out.push_sql(" = ");
        out.push_sql(alias);
        out.push_sql(".");
        BatchUpdateTarget::column_target(self, out.reborrow())
    }
}

impl<Col, U, DB> BatchUpdateExpr<DB, Col::Table> for Assign<ColumnWrapperForUpdate<Col>, U>
where
    DB: Backend,
    Col: Column,
    U: QueryFragment<DB>,
    Self: QueryFragment<DB>,
{
    fn column_expr<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        QueryFragment::walk_ast(&self.expr, out.reborrow())
    }
}

#[derive(Debug)]
pub struct BatchChangeSet<T, Tab> {
    pub(crate) values: Vec<T>,
    _marker: PhantomData<Tab>,
}

impl<T, Tab, DB> QueryFragment<DB> for BatchChangeSet<T, Tab>
where
    DB: Backend,
    Tab: Table,
    T: BatchUpdateTargetAssign<DB, Tab> + BatchUpdateExpr<DB, Tab> + Clone,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        if let None = self.values.first() {
            // TODO: more meaningful return maybe?
            return Ok(());
        }

        // TODO: Create proper struct for alias that implements QueryFragment.
        // Maybe there is already something?
        const ALIAS: &str = "\"tmp\"";

        // --- Create this statement with the following steps:
        // UPDATE my_table AS tab SET
        //     column_a = tmp.column_a,
        //     column_c = tmp.column_c
        // FROM ( VALUES
        //     ('aa', 1, 11),
        //     ('bb', 2, 22)
        // ) AS tmp(column_a, column_b, column_c)
        // WHERE tab.column_b = tmp.column_b;

        // --- Assign columns to temporary columns
        //     column_a = tmp.column_a,
        //     column_c = tmp.column_c
        if let Some(first) = self.values.first() {
            first.column_target_assign(out.reborrow(), ALIAS)?;
        }

        // --- List of values
        // FROM ( VALUES
        //     ('aa', 1, 11),
        //     ('bb', 2, 22)
        // )
        out.push_sql(" FROM ( VALUES");
        let mut values = self.values.iter();
        if let Some(value) = values.next() {
            out.push_sql(" (");
            value.column_expr(out.reborrow())?;
            out.push_sql(")");
        }
        for value in values {
            out.push_sql(", (");
            value.column_expr(out.reborrow())?;
            out.push_sql(")");
        }
        out.push_sql(" )");

        // --- Set alias and its columns
        //     AS tmp(column_a, column_b, column_c)
        if let Some(last) = self.values.last() {
            out.push_sql(" AS ");
            out.push_sql(ALIAS);
            out.push_sql("(");
            last.column_target(out.reborrow())?;
            out.push_sql(")");
        }

        // id = primary key -> therefore not available in update batch! See Note below.
        const UPDATE_CONDITION_COLUMN: &str = "\"name\"";
        // --- TODO: Handle proper UpdateStatement::where_clause
        // --- Implemented here for testing purpose!
        // WHERE tab.column_b = tmp.column_b;
        out.push_sql(" WHERE \"users\".");
        out.push_sql(UPDATE_CONDITION_COLUMN);
        out.push_sql(" = ");
        out.push_sql(ALIAS);
        out.push_sql(".");
        out.push_sql(UPDATE_CONDITION_COLUMN);

        /*
        NOTE: derive(AsChangeset) prevents from updating the primary key.
        Therefore the primary key cannot be provided in the update batch
        and then referenced in the WHERE clause.
        Both User_A and User_B will create the alias:
            AS tmp(name, hair_color, type)

        struct User_A {
            name: String,
            hair_color: String,
            r#type: String,
        }

        struct User_B {
            id: u32,
            name: String,
            hair_color: String,
            r#type: String,
        }
        */

        Ok(())
    }
}
