#[macro_export]
#[doc(hidden)]
macro_rules! infix_predicate_body {
    ($name:ident, $operator:expr, $return_type:ty) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name<T, U> {
            left: T,
            right: U,
        }

        impl<T, U> $name<T, U> {
            pub fn new(left: T, right: U) -> Self {
                $name {
                    left: left,
                    right: right,
                }
            }
        }

        impl<T, U> $crate::expression::Expression for $name<T, U> where
            T: $crate::expression::Expression,
            U: $crate::expression::Expression,
        {
            type SqlType = $return_type;
        }

        impl<T, U, DB> $crate::query_builder::QueryFragment<DB> for $name<T, U> where
            DB: $crate::backend::Backend,
            T: $crate::query_builder::QueryFragment<DB>,
            U: $crate::query_builder::QueryFragment<DB>,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                try!(self.left.to_sql(out));
                out.push_sql($operator);
                self.right.to_sql(out)
            }
        }

        impl<T, U, QS> $crate::expression::SelectableExpression<QS> for $name<T, U> where
            T: $crate::expression::SelectableExpression<QS>,
            U: $crate::expression::SelectableExpression<QS>,
        {
        }

        impl<T, U> $crate::expression::NonAggregate for $name<T, U> where
            T: $crate::expression::NonAggregate,
            U: $crate::expression::NonAggregate,
        {
        }
    }
}

#[macro_export]
/// Useful for libraries adding support for new SQL types. Apps should never
/// need to call this
///
/// # Example
///
/// ```ignore
/// infix_predicate!(Matches, " @@ ");
/// infix_predicate!(Concat, " || ", TsVector);
/// infix_predicate!(And, " && ", TsQuery);
/// infix_predicate!(Or, " || ", TsQuery);
/// infix_predicate!(Contains, " @> ");
/// infix_predicate!(ContainedBy, " @> ");
/// ```
macro_rules! infix_predicate {
    ($name:ident, $operator:expr) => {
        infix_predicate!($name, $operator, $crate::types::Bool);
    };

    ($name:ident, $operator:expr, $return_type:ty) => {
        infix_predicate_body!($name, $operator, $return_type);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! postfix_predicate_body {
    ($name:ident, $operator:expr, $return_type:ty) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name<T> {
            expr: T,
        }

        impl<T> $name<T> {
            pub fn new(expr: T) -> Self {
                $name {
                    expr: expr,
                }
            }
        }

        impl<T> $crate::expression::Expression for $name<T> where
            T: $crate::expression::Expression,
        {
            type SqlType = $return_type;
        }

        impl<T, DB> $crate::query_builder::QueryFragment<DB> for $name<T> where
            DB: $crate::backend::Backend,
            T: $crate::query_builder::QueryFragment<DB>,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                try!(self.expr.to_sql(out));
                out.push_sql($operator);
                Ok(())
            }
        }

        impl<T, QS> $crate::expression::SelectableExpression<QS> for $name<T> where
            T: $crate::expression::SelectableExpression<QS>,
        {
        }

        impl<T> $crate::expression::NonAggregate for $name<T> where
            T: $crate::expression::NonAggregate,
        {
        }
    }
}

#[macro_export]
/// Useful for libraries adding support for new SQL types. Apps should never
/// need to call this.
macro_rules! postfix_predicate {
    ($name:ident, $operator:expr) => {
        postfix_expression!($name, $operator, $crate::types::Bool);
    };
}

#[macro_export]
macro_rules! postfix_expression {
    ($name:ident, $operator:expr, $return_type:ty) => {
        postfix_predicate_body!($name, $operator, $return_type);
    }
}

infix_predicate!(And, " AND ");
infix_predicate!(Between, " BETWEEN ");
infix_predicate!(Eq, " = ");
infix_predicate!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ");
infix_predicate!(Gt, " > ");
infix_predicate!(GtEq, " >= ");
infix_predicate!(Like, " LIKE ");
infix_predicate!(Lt, " < ");
infix_predicate!(LtEq, " <= ");
infix_predicate!(NotBetween, " NOT BETWEEN ");
infix_predicate!(NotEq, " != ");
infix_predicate!(NotLike, " NOT LIKE ");
infix_predicate!(Or, " OR ");

postfix_predicate!(IsNull, " IS NULL");
postfix_predicate!(IsNotNull, " IS NOT NULL");
postfix_expression!(Asc, " ASC", ());
postfix_expression!(Desc, " DESC", ());

use backend::Backend;
use query_source::Column;
use query_builder::*;
use super::SelectableExpression;

impl<T, U, DB> Changeset<DB> for Eq<T, U> where
    DB: Backend,
    T: Column,
    U: SelectableExpression<T::Table> + QueryFragment<DB>,
{
    fn is_noop(&self) -> bool {
        false
    }

    fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
        try!(out.push_identifier(T::name()));
        out.push_sql(" = ");
        QueryFragment::to_sql(&self.right, out)
    }
}

impl<T, U> AsChangeset for Eq<T, U> where
    T: Column,
    U: SelectableExpression<T::Table>,
{
    type Target = T::Table;
    type Changeset = Self;

    fn as_changeset(self) -> Self {
        self
    }
}
