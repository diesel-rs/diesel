//! Types related to describing schema, and interactions between tables.
//!
//! Most traits in this module are derived or generated by [`table!`].
//!
//! [`table!`]: ../macro.table.html
#[doc(hidden)]
pub mod joins;
mod peano_numbers;

use crate::expression::{Expression, SelectableExpression, ValidGrouping};
use crate::query_builder::*;

pub use self::joins::JoinTo;
pub use self::peano_numbers::*;

/// Represents a type which can appear in the `FROM` clause. Apps should not
/// need to concern themselves with this trait.
///
/// Types which implement this trait include:
/// - Tables generated by the `table!` macro
/// - Internal structs which represent joins
/// - A select statement which has had no query builder methods called on it,
///   other than those which can affect the from clause.
pub trait QuerySource {
    /// The type returned by `from_clause`
    type FromClause;
    /// The type returned by `default_selection`
    type DefaultSelection: SelectableExpression<Self>;

    /// The actual `FROM` clause of this type. This is typically only called in
    /// `QueryFragment` implementations.
    fn from_clause(&self) -> Self::FromClause;
    /// The default select clause of this type, which should be used if no
    /// select clause was explicitly specified. This should always be a tuple of
    /// all the desired columns, not `star`
    fn default_selection(&self) -> Self::DefaultSelection;
}

/// A column on a database table. Types which implement this trait should have
/// been generated by the [`table!` macro](../macro.table.html).
pub trait Column: Expression {
    /// The table which this column belongs to
    type Table: Table;

    /// The name of this column
    const NAME: &'static str;

    ///是否为自增ID
    const SELF_INCREASE_ID : bool;

    ///判断是否为自增ID
    fn is_self_increase_id(&self)->bool{
        Self::SELF_INCREASE_ID        
    }

    ///获取列名
    fn name(&self)->&'static str{
        Self::NAME
    }
    
    ///获取列类型
    fn sql_type(&self)->String;

}

/// A SQL database table. Types which implement this trait should have been
/// generated by the [`table!` macro](../macro.table.html).
pub trait Table: QuerySource + AsQuery + Sized {
    /// The type returned by `primary_key`
    type PrimaryKey: SelectableExpression<Self> + ValidGrouping<()>;
    /// The type returned by `all_columns`
    type AllColumns: SelectableExpression<Self> + ValidGrouping<()>;

    /// Returns the primary key of this table.
    ///
    /// If the table has a composite primary key, this will be a tuple.
    fn primary_key(&self) -> Self::PrimaryKey;
    /// Returns a tuple of all columns belonging to this table.
    fn all_columns() -> Self::AllColumns;
}

/// Determines how many times `Self` appears in `QS`
///
/// This trait is primarily used to determine whether or not a column is
/// selectable from a given from clause. A column can be selected if its table
/// appears in the from clause *exactly once*.
///
/// We do not allow the same table to appear in a query multiple times in any
/// context where referencing that table would be ambiguous (depending on the
/// context and backend being used, this may or may not be something that would
/// otherwise result in a runtime error).
pub trait AppearsInFromClause<QS> {
    /// How many times does `Self` appear in `QS`?
    type Count;
}

#[doc(hidden)]
/// Used to determine which of two from clauses contains a given table.
///
/// This trait can be used to emulate "or" conditions in where clauses when
/// we want a trait to be implemented with one of two type parameters.
///
/// For example, if we wanted to write:
///
/// ```rust,ignore
/// where
///     T: SelectableExpression<Left> | SelectableExpression<Right>,
/// ```
///
/// we can emulate this by writing:
///
/// ```rust,ignore
/// where
///     Left: AppearsInFromClause<T::Table>,
///     Right: AppearsInFromClause<T::Table>,
///     (Left::Count, Right::Count): Pick<Left, Right>,
///     T: SelectableExpression<
///         <(Left::Count, Right::Count) as Pick<Left, Right>>::Selection,
///     >,
/// ```
///
/// In order to aquire the counts in the first place, we must already know
/// the table we're searching for.
pub trait Pick<Left, Right> {
    /// The selected type.
    ///
    /// For `(Once, Never)` this type will be `Left`. For `(Never, Once)`, this type will be
    /// `Right`
    type Selection;
}

impl<Left, Right> Pick<Left, Right> for (Once, Never) {
    type Selection = Left;
}

impl<Left, Right> Pick<Left, Right> for (Never, Once) {
    type Selection = Right;
}

#[doc(hidden)]
#[allow(
    non_camel_case_types,
    missing_debug_implementations,
    missing_copy_implementations
)]
/// Everything in this module is here to give something more helpful than:
///
/// > (Never, Never): Pick<table1, table2> is not satisifed
///
/// Any of these impls can be deleted if they are getting in the way of
/// other functionality. Any code which is using these impls is already
/// failing to compile.
mod impls_which_are_only_here_to_improve_error_messages {
    use super::*;

    pub struct this_table_doesnt_appear_in_the_from_clause_of_your_query;

    impl<Left, Right> Pick<Left, Right> for (Never, Never) {
        type Selection = this_table_doesnt_appear_in_the_from_clause_of_your_query;
    }

    pub struct this_table_appears_in_your_query_more_than_once_and_must_be_aliased;

    impl<Left, Right, OtherCount> Pick<Left, Right> for (MoreThanOnce, OtherCount) {
        type Selection = this_table_appears_in_your_query_more_than_once_and_must_be_aliased;
    }

    impl<Left, Right> Pick<Left, Right> for (Never, MoreThanOnce) {
        type Selection = this_table_appears_in_your_query_more_than_once_and_must_be_aliased;
    }

    impl<Left, Right> Pick<Left, Right> for (Once, MoreThanOnce) {
        type Selection = this_table_appears_in_your_query_more_than_once_and_must_be_aliased;
    }
}
