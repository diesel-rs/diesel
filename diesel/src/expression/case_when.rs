use crate::expression::grouped::Grouped;
use crate::expression::helper_types::case_when_else;
use crate::expression::Expression;
use crate::sql_types::{BoolOrNullableBool, SqlType};

use super::{AsExpression, TypedExpressionType};

/// Creates a SQL `CASE WHEN ... ELSE ... END` expression
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// use diesel::dsl::case_when_else;
///
/// let users_with_name: Vec<(i32, i32)> = users
///     .select((id, case_when_else(name.eq("Sean"), id, 0)))
///     .load(connection)
///     .unwrap();
///
/// assert_eq!(&[(1, 1), (2, 0)], users_with_name.as_slice());
/// # }
/// ```
///
/// Note that the SQL types of the `if_true` and `if_false` expressions should
/// be equal. This includes whether they are wrapped in
/// [`Nullable`](crate::sql_types::Nullable), so you may need to call
/// [`nullable`](crate::expression_methods::NullableExpressionMethods::nullable)
/// on one of them.
pub fn case_when_else<C, T, F, ST>(
    condition: C,
    if_true: T,
    if_false: F,
) -> case_when_else<C, T, F, ST>
where
    C: Expression,
    <C as Expression>::SqlType: BoolOrNullableBool,
    T: AsExpression<ST>,
    F: AsExpression<ST>,
    ST: SqlType + TypedExpressionType,
{
    CaseWhenElse {
        condition: Grouped(condition),
        if_true: Grouped(if_true.as_expression()),
        if_false: Grouped(if_false.as_expression()),
    }
}

pub(crate) use case_when_else_impl::CaseWhenElse;
mod case_when_else_impl {
    use diesel_derives::{DieselNumericOps, QueryId, ValidGrouping};

    use crate::expression::{AppearsOnTable, Expression, SelectableExpression};
    use crate::query_builder::{AstPass, QueryFragment};
    use crate::query_source::aliasing;
    use crate::sql_types::BoolOrNullableBool;

    #[derive(Debug, Clone, Copy, QueryId, DieselNumericOps, ValidGrouping)]
    pub struct CaseWhenElse<C, T, F> {
        pub(super) condition: C,
        pub(super) if_true: T,
        pub(super) if_false: F,
    }

    impl<C, T, F, QS> SelectableExpression<QS> for CaseWhenElse<C, T, F>
    where
        CaseWhenElse<C, T, F>: AppearsOnTable<QS>,
        C: SelectableExpression<QS>,
        T: SelectableExpression<QS>,
        F: SelectableExpression<QS>,
    {
    }

    impl<C, T, F, QS> AppearsOnTable<QS> for CaseWhenElse<C, T, F>
    where
        CaseWhenElse<C, T, F>: Expression,
        C: AppearsOnTable<QS>,
        T: AppearsOnTable<QS>,
        F: AppearsOnTable<QS>,
    {
    }

    impl<C, T, F> Expression for CaseWhenElse<C, T, F>
    where
        C: Expression,
        <C as Expression>::SqlType: BoolOrNullableBool,
        T: Expression,
        F: Expression<SqlType = <T as Expression>::SqlType>,
    {
        type SqlType = <T as Expression>::SqlType;
    }
    impl<C, T, F, DB> QueryFragment<DB> for CaseWhenElse<C, T, F>
    where
        C: QueryFragment<DB>,
        T: QueryFragment<DB>,
        F: QueryFragment<DB>,
        DB: crate::backend::Backend,
    {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> crate::result::QueryResult<()> {
            out.push_sql("CASE WHEN ");
            self.condition.walk_ast(out.reborrow())?;
            out.push_sql(" THEN ");
            self.if_true.walk_ast(out.reborrow())?;
            out.push_sql(" ELSE ");
            self.if_false.walk_ast(out.reborrow())?;
            out.push_sql(" END");
            Ok(())
        }
    }
    impl<S, C, T, F> aliasing::FieldAliasMapper<S> for CaseWhenElse<C, T, F>
    where
        S: aliasing::AliasSource,
        C: aliasing::FieldAliasMapper<S>,
        T: aliasing::FieldAliasMapper<S>,
        F: aliasing::FieldAliasMapper<S>,
    {
        type Out = CaseWhenElse<
            <C as aliasing::FieldAliasMapper<S>>::Out,
            <T as aliasing::FieldAliasMapper<S>>::Out,
            <F as aliasing::FieldAliasMapper<S>>::Out,
        >;
        fn map(self, alias: &aliasing::Alias<S>) -> Self::Out {
            CaseWhenElse {
                condition: self.condition.map(alias),
                if_true: self.if_true.map(alias),
                if_false: self.if_false.map(alias),
            }
        }
    }
}
