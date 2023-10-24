use crate::expression::grouped::Grouped;
use crate::expression::{helper_types, Expression};
use crate::sql_types::{BoolOrNullableBool, SqlType};
use diesel_derives::{DieselNumericOps, QueryId, ValidGrouping};

use super::{AsExpression, TypedExpressionType};

/// Creates a SQL `CASE WHEN ... END` expression
///
/// # Example
///
/// ```
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// use diesel::dsl::case_when;
///
/// let users_with_name: Vec<(i32, Option<i32>)> = users
///     .select((id, case_when(name.eq("Sean"), id)))
///     .load(connection)
///     .unwrap();
///
/// assert_eq!(&[(1, Some(1)), (2, None)], users_with_name.as_slice());
/// # }
/// ```
///
/// # `ELSE` clause
/// ```
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// use diesel::dsl::case_when;
///
/// let users_with_name: Vec<(i32, i32)> = users
///     .select((id, case_when(name.eq("Sean"), id).otherwise(0)))
///     .load(connection)
///     .unwrap();
///
/// assert_eq!(&[(1, 1), (2, 0)], users_with_name.as_slice());
/// # }
/// ```
///
/// Note that the SQL types of the `case_when` and `else` expressions should
/// be equal. This includes whether they are wrapped in
/// [`Nullable`](crate::sql_types::Nullable), so you may need to call
/// [`nullable`](crate::expression_methods::NullableExpressionMethods::nullable)
/// on one of them.
///
/// # More `WHEN` branches
/// ```
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// use diesel::dsl::case_when;
///
/// let users_with_name: Vec<(i32, Option<i32>)> = users
///     .select((id, case_when(name.eq("Sean"), id).when(name.eq("Tess"), 2)))
///     .load(connection)
///     .unwrap();
///
/// assert_eq!(&[(1, Some(1)), (2, Some(2))], users_with_name.as_slice());
/// # }
/// ```
pub fn case_when<C, T, ST>(condition: C, if_true: T) -> helper_types::case_when<C, T, ST>
where
    C: Expression,
    <C as Expression>::SqlType: BoolOrNullableBool,
    T: AsExpression<ST>,
    ST: SqlType + TypedExpressionType,
{
    CaseWhen {
        whens: CaseWhenConditionsLeaf {
            when: Grouped(condition),
            then: Grouped(if_true.as_expression()),
        },
        else_expr: NoElseExpression,
    }
}

/// A SQL `CASE WHEN ... END` expression
#[derive(Debug, Clone, Copy, QueryId, DieselNumericOps, ValidGrouping)]
pub struct CaseWhen<Whens, E> {
    whens: Whens,
    else_expr: E,
}

impl<Whens, E> CaseWhen<Whens, E> {
    /// Add an additional `WHEN ... THEN ...` branch to the `CASE` expression
    ///
    /// See the [`case_when`] documentation for more details.
    pub fn when<C, T>(self, condition: C, if_true: T) -> helper_types::When<Self, C, T>
    where
        Self: CaseWhenTypesExtractor<Whens = Whens, Else = E>,
        C: Expression,
        <C as Expression>::SqlType: BoolOrNullableBool,
        T: AsExpression<<Self as CaseWhenTypesExtractor>::OutputExpressionSpecifiedSqlType>,
    {
        CaseWhen {
            whens: CaseWhenConditionsIntermediateNode {
                first_whens: self.whens,
                last_when: CaseWhenConditionsLeaf {
                    when: Grouped(condition),
                    then: Grouped(if_true.as_expression()),
                },
            },
            else_expr: self.else_expr,
        }
    }
}

impl<Whens> CaseWhen<Whens, NoElseExpression> {
    /// Sets the `ELSE` branch of the `CASE` expression
    ///
    /// It is named this way because `else` is a reserved keyword in Rust
    ///
    /// See the [`case_when`] documentation for more details.
    pub fn otherwise<E>(self, if_no_other_branch_matched: E) -> helper_types::Otherwise<Self, E>
    where
        Self: CaseWhenTypesExtractor<Whens = Whens, Else = NoElseExpression>,
        E: AsExpression<<Self as CaseWhenTypesExtractor>::OutputExpressionSpecifiedSqlType>,
    {
        CaseWhen {
            whens: self.whens,
            else_expr: ElseExpression {
                expr: Grouped(if_no_other_branch_matched.as_expression()),
            },
        }
    }
}

pub(crate) use non_public_types::*;
mod non_public_types {
    use super::CaseWhen;

    use diesel_derives::{QueryId, ValidGrouping};

    use crate::expression::{
        AppearsOnTable, Expression, SelectableExpression, TypedExpressionType,
    };
    use crate::query_builder::{AstPass, QueryFragment};
    use crate::query_source::aliasing;
    use crate::sql_types::{BoolOrNullableBool, IntoNullable, SqlType};

    #[derive(Debug, Clone, Copy, QueryId, ValidGrouping)]
    pub struct CaseWhenConditionsLeaf<W, T> {
        pub(super) when: W,
        pub(super) then: T,
    }

    #[derive(Debug, Clone, Copy, QueryId, ValidGrouping)]
    pub struct CaseWhenConditionsIntermediateNode<W, T, Whens> {
        pub(super) first_whens: Whens,
        pub(super) last_when: CaseWhenConditionsLeaf<W, T>,
    }

    pub trait CaseWhenConditions {
        type OutputExpressionSpecifiedSqlType: SqlType + TypedExpressionType;
    }
    impl<W, T: Expression> CaseWhenConditions for CaseWhenConditionsLeaf<W, T>
    where
        <T as Expression>::SqlType: SqlType + TypedExpressionType,
    {
        type OutputExpressionSpecifiedSqlType = T::SqlType;
    }
    // This intentionally doesn't re-check inner `Whens` here, because this trait is
    // only used to allow expression SQL type inference for `.when` calls so we
    // want to make it as lightweight as possible for fast compilation. Actual
    // guarantees are provided by the other implementations below
    impl<W, T: Expression, Whens> CaseWhenConditions for CaseWhenConditionsIntermediateNode<W, T, Whens>
    where
        <T as Expression>::SqlType: SqlType + TypedExpressionType,
    {
        type OutputExpressionSpecifiedSqlType = T::SqlType;
    }

    #[derive(Debug, Clone, Copy, QueryId, ValidGrouping)]
    pub struct NoElseExpression;
    #[derive(Debug, Clone, Copy, QueryId, ValidGrouping)]
    pub struct ElseExpression<E> {
        pub(super) expr: E,
    }

    /// Largely internal trait used to define the [`When`] and [`Otherwise`]
    /// type aliases
    ///
    /// It should typically not be needed in user code unless writing extremely
    /// generic functions
    pub trait CaseWhenTypesExtractor {
        /// The
        /// This may not be the actual output expression type: if there is no
        /// `else` it will be made `Nullable`
        type OutputExpressionSpecifiedSqlType: SqlType + TypedExpressionType;
        type Whens;
        type Else;
    }
    impl<Whens, E> CaseWhenTypesExtractor for CaseWhen<Whens, E>
    where
        Whens: CaseWhenConditions,
    {
        type OutputExpressionSpecifiedSqlType = Whens::OutputExpressionSpecifiedSqlType;
        type Whens = Whens;
        type Else = E;
    }

    impl<W, T, QS> SelectableExpression<QS> for CaseWhen<CaseWhenConditionsLeaf<W, T>, NoElseExpression>
    where
        CaseWhen<CaseWhenConditionsLeaf<W, T>, NoElseExpression>: AppearsOnTable<QS>,
        W: SelectableExpression<QS>,
        T: SelectableExpression<QS>,
    {
    }

    impl<W, T, E, QS> SelectableExpression<QS>
        for CaseWhen<CaseWhenConditionsLeaf<W, T>, ElseExpression<E>>
    where
        CaseWhen<CaseWhenConditionsLeaf<W, T>, ElseExpression<E>>: AppearsOnTable<QS>,
        W: SelectableExpression<QS>,
        T: SelectableExpression<QS>,
        E: SelectableExpression<QS>,
    {
    }

    impl<W, T, Whens, E, QS> SelectableExpression<QS>
        for CaseWhen<CaseWhenConditionsIntermediateNode<W, T, Whens>, E>
    where
        Self: AppearsOnTable<QS>,
        W: SelectableExpression<QS>,
        T: SelectableExpression<QS>,
        CaseWhen<Whens, E>: SelectableExpression<QS>,
    {
    }

    impl<W, T, QS> AppearsOnTable<QS> for CaseWhen<CaseWhenConditionsLeaf<W, T>, NoElseExpression>
    where
        CaseWhen<CaseWhenConditionsLeaf<W, T>, NoElseExpression>: Expression,
        W: AppearsOnTable<QS>,
        T: AppearsOnTable<QS>,
    {
    }

    impl<W, T, E, QS> AppearsOnTable<QS> for CaseWhen<CaseWhenConditionsLeaf<W, T>, ElseExpression<E>>
    where
        CaseWhen<CaseWhenConditionsLeaf<W, T>, ElseExpression<E>>: Expression,
        W: AppearsOnTable<QS>,
        T: AppearsOnTable<QS>,
        E: AppearsOnTable<QS>,
    {
    }

    impl<W, T, Whens, E, QS> AppearsOnTable<QS>
        for CaseWhen<CaseWhenConditionsIntermediateNode<W, T, Whens>, E>
    where
        Self: Expression,
        W: AppearsOnTable<QS>,
        T: AppearsOnTable<QS>,
        CaseWhen<Whens, E>: AppearsOnTable<QS>,
    {
    }

    impl<W, T> Expression for CaseWhen<CaseWhenConditionsLeaf<W, T>, NoElseExpression>
    where
        W: Expression,
        <W as Expression>::SqlType: BoolOrNullableBool,
        T: Expression,
        <T as Expression>::SqlType: IntoNullable,
        <<T as Expression>::SqlType as IntoNullable>::Nullable: SqlType + TypedExpressionType,
    {
        type SqlType = <<T as Expression>::SqlType as IntoNullable>::Nullable;
    }
    impl<W, T, E> Expression for CaseWhen<CaseWhenConditionsLeaf<W, T>, ElseExpression<E>>
    where
        W: Expression,
        <W as Expression>::SqlType: BoolOrNullableBool,
        T: Expression,
    {
        type SqlType = T::SqlType;
    }
    impl<W, T, Whens, E> Expression for CaseWhen<CaseWhenConditionsIntermediateNode<W, T, Whens>, E>
    where
        CaseWhen<CaseWhenConditionsLeaf<W, T>, E>: Expression,
        CaseWhen<Whens, E>: Expression<
            SqlType = <CaseWhen<CaseWhenConditionsLeaf<W, T>, E> as Expression>::SqlType,
        >,
    {
        type SqlType = <CaseWhen<CaseWhenConditionsLeaf<W, T>, E> as Expression>::SqlType;
    }

    impl<Whens, E, DB> QueryFragment<DB> for CaseWhen<Whens, E>
    where
        DB: crate::backend::Backend,
        Whens: QueryFragment<DB>,
        E: QueryFragment<DB>,
    {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> crate::QueryResult<()> {
            out.push_sql("CASE");
            self.whens.walk_ast(out.reborrow())?;
            self.else_expr.walk_ast(out.reborrow())?;
            out.push_sql(" END");
            Ok(())
        }
    }

    impl<W, T, DB> QueryFragment<DB> for CaseWhenConditionsLeaf<W, T>
    where
        DB: crate::backend::Backend,
        W: QueryFragment<DB>,
        T: QueryFragment<DB>,
    {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> crate::QueryResult<()> {
            out.push_sql(" WHEN ");
            self.when.walk_ast(out.reborrow())?;
            out.push_sql(" THEN ");
            self.then.walk_ast(out.reborrow())?;
            Ok(())
        }
    }

    impl<W, T, Whens, DB> QueryFragment<DB> for CaseWhenConditionsIntermediateNode<W, T, Whens>
    where
        DB: crate::backend::Backend,
        Whens: QueryFragment<DB>,
        W: QueryFragment<DB>,
        T: QueryFragment<DB>,
    {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> crate::QueryResult<()> {
            self.first_whens.walk_ast(out.reborrow())?;
            self.last_when.walk_ast(out.reborrow())?;
            Ok(())
        }
    }

    impl<DB> QueryFragment<DB> for NoElseExpression
    where
        DB: crate::backend::Backend,
    {
        fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> crate::result::QueryResult<()> {
            let _ = out;
            Ok(())
        }
    }
    impl<E, DB> QueryFragment<DB> for ElseExpression<E>
    where
        E: QueryFragment<DB>,
        DB: crate::backend::Backend,
    {
        fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> crate::result::QueryResult<()> {
            out.push_sql(" ELSE ");
            self.expr.walk_ast(out.reborrow())?;
            Ok(())
        }
    }

    impl<S, Conditions, E> aliasing::FieldAliasMapper<S> for CaseWhen<Conditions, E>
    where
        S: aliasing::AliasSource,
        Conditions: aliasing::FieldAliasMapper<S>,
        E: aliasing::FieldAliasMapper<S>,
    {
        type Out = CaseWhen<
            <Conditions as aliasing::FieldAliasMapper<S>>::Out,
            <E as aliasing::FieldAliasMapper<S>>::Out,
        >;
        fn map(self, alias: &aliasing::Alias<S>) -> Self::Out {
            CaseWhen {
                whens: self.whens.map(alias),
                else_expr: self.else_expr.map(alias),
            }
        }
    }

    impl<S, W, T> aliasing::FieldAliasMapper<S> for CaseWhenConditionsLeaf<W, T>
    where
        S: aliasing::AliasSource,
        W: aliasing::FieldAliasMapper<S>,
        T: aliasing::FieldAliasMapper<S>,
    {
        type Out = CaseWhenConditionsLeaf<
            <W as aliasing::FieldAliasMapper<S>>::Out,
            <T as aliasing::FieldAliasMapper<S>>::Out,
        >;
        fn map(self, alias: &aliasing::Alias<S>) -> Self::Out {
            CaseWhenConditionsLeaf {
                when: self.when.map(alias),
                then: self.then.map(alias),
            }
        }
    }

    impl<S, W, T, Whens> aliasing::FieldAliasMapper<S>
        for CaseWhenConditionsIntermediateNode<W, T, Whens>
    where
        S: aliasing::AliasSource,
        W: aliasing::FieldAliasMapper<S>,
        T: aliasing::FieldAliasMapper<S>,
        Whens: aliasing::FieldAliasMapper<S>,
    {
        type Out = CaseWhenConditionsIntermediateNode<
            <W as aliasing::FieldAliasMapper<S>>::Out,
            <T as aliasing::FieldAliasMapper<S>>::Out,
            <Whens as aliasing::FieldAliasMapper<S>>::Out,
        >;
        fn map(self, alias: &aliasing::Alias<S>) -> Self::Out {
            CaseWhenConditionsIntermediateNode {
                first_whens: self.first_whens.map(alias),
                last_when: self.last_when.map(alias),
            }
        }
    }
}
