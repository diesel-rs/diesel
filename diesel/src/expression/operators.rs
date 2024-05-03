#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_operator_body {
    (
        notation = $notation:ident,
        struct_name = $name:ident,
        operator = $operator:expr,
        return_ty = (ReturnBasedOnArgs),
        ty_params = ($($ty_param:ident,)+),
        field_names = $field_names:tt,
        backend_ty_params = $backend_ty_params:tt,
        backend_ty = $backend_ty:ty,
    ) => {
        $crate::__diesel_operator_body! {
            notation = $notation,
            struct_name = $name,
            operator = $operator,
            return_ty = (ST),
            ty_params = ($($ty_param,)+),
            field_names = $field_names,
            backend_ty_params = $backend_ty_params,
            backend_ty = $backend_ty,
            expression_ty_params = (ST,),
            expression_bounds = ($($ty_param: $crate::expression::Expression<SqlType = ST>,)+),
        }
    };

    (
        notation = $notation:ident,
        struct_name = $name:ident,
        operator = $operator:expr,
        return_ty = ($($return_ty:tt)+),
        ty_params = ($($ty_param:ident,)+),
        field_names = $field_names:tt,
        backend_ty_params = $backend_ty_params:tt,
        backend_ty = $backend_ty:ty,
    ) => {
        $crate::__diesel_operator_body! {
            notation = $notation,
            struct_name = $name,
            operator = $operator,
            return_ty = ($($return_ty)*),
            ty_params = ($($ty_param,)+),
            field_names = $field_names,
            backend_ty_params = $backend_ty_params,
            backend_ty = $backend_ty,
            expression_ty_params = (),
            expression_bounds = ($($ty_param: $crate::expression::Expression,)+),
        }
    };

    (
        notation = $notation:ident,
        struct_name = $name:ident,
        operator = $operator:expr,
        return_ty = ($($return_ty:tt)+),
        ty_params = ($($ty_param:ident,)+),
        field_names = ($($field_name:ident,)+),
        backend_ty_params = ($($backend_ty_param:ident,)*),
        backend_ty = $backend_ty:ty,
        expression_ty_params = ($($expression_ty_params:ident,)*),
        expression_bounds = ($($expression_bounds:tt)*),
    ) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            $crate::query_builder::QueryId,
            $crate::sql_types::DieselNumericOps,
            $crate::expression::ValidGrouping
        )]
        #[doc(hidden)]
        pub struct $name<$($ty_param,)+> {
            $(pub(crate) $field_name: $ty_param,)+
        }

        impl<$($ty_param,)+> $name<$($ty_param,)+> {
            pub(crate) fn new($($field_name: $ty_param,)+) -> Self {
                $name { $($field_name,)+ }
            }
        }

        $crate::impl_selectable_expression!($name<$($ty_param),+>);

        impl<$($ty_param,)+ $($expression_ty_params,)*> $crate::expression::Expression for $name<$($ty_param,)+> where
            $($expression_bounds)*
        {
            type SqlType = $($return_ty)*;
        }

        impl<$($ty_param,)+ $($backend_ty_param,)*> $crate::query_builder::QueryFragment<$backend_ty>
            for $name<$($ty_param,)+> where
                $($ty_param: $crate::query_builder::QueryFragment<$backend_ty>,)+
                $($backend_ty_param: $crate::backend::Backend,)*
        {
            fn walk_ast<'b>(
                &'b self,
                mut out: $crate::query_builder::AstPass<'_, 'b, $backend_ty>
            ) -> $crate::result::QueryResult<()>
            {
                $crate::__diesel_operator_to_sql!(
                    notation = $notation,
                    operator_expr = out.push_sql($operator),
                    field_exprs = ($(self.$field_name.walk_ast(out.reborrow())?),+),
                );
                Ok(())
            }
        }

        impl<S, $($ty_param,)+> $crate::internal::operators_macro::FieldAliasMapper<S> for $name<$($ty_param,)+>
        where
            S: $crate::query_source::AliasSource,
            $($ty_param: $crate::internal::operators_macro::FieldAliasMapper<S>,)+
        {
            type Out = $name<
                $(<$ty_param as $crate::internal::operators_macro::FieldAliasMapper<S>>::Out,)+
            >;
            fn map(self, alias: &$crate::query_source::Alias<S>) -> Self::Out {
                $name {
                    $($field_name: self.$field_name.map(alias),)+
                }
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_operator_to_sql {
    (
        notation = infix,
        operator_expr = $op:expr,
        field_exprs = ($left:expr, $right:expr),
    ) => {
        $left;
        $op;
        $right;
    };

    (
        notation = postfix,
        operator_expr = $op:expr,
        field_exprs = ($expr:expr),
    ) => {
        $expr;
        $op;
    };

    (
        notation = prefix,
        operator_expr = $op:expr,
        field_exprs = ($expr:expr),
    ) => {
        $op;
        $expr;
    };
}

/// Useful for libraries adding support for new SQL types. Apps should never
/// need to call this.
///
/// This will create a new type with the given name. It will implement all
/// methods needed to be used as an expression in Diesel, placing the given
/// SQL between the two elements. The third argument specifies the SQL type
/// that the operator returns. If it is not given, the type will be assumed
/// to be `Bool`.
///
/// If the operator is specific to a single backend, you can specify this by
/// adding `backend: Pg` or similar as the last argument.
///
/// It should be noted that the generated impls will not constrain the SQL
/// types of the arguments. You should ensure that they are of the right
/// type in your function which constructs the operator.
///
/// Typically you would not expose the type that this generates directly. You'd
/// expose a function (or trait) used to construct the expression, and a helper
/// type which represents the return type of that function. See the source of
/// `diesel::expression::expression_methods` and
/// `diesel::expression::helper_types` for real world examples of this.
///
/// # Examples
///
/// # Possible invocations
///
/// ```ignore
/// // The SQL type will be boolean. The backend will not be constrained
/// infix_operator!(Matches, " @@ ");
///
/// // Queries which try to execute `Contains` on a backend other than Pg
/// // will fail to compile
/// infix_operator!(Contains, " @> ", backend: Pg);
///
/// // The type of `Concat` will be `TsVector` rather than Bool
/// infix_operator!(Concat, " || ", TsVector);
///
/// // It is perfectly fine to have multiple operators with the same SQL.
/// // Diesel will ensure that the queries are always unambiguous in which
/// // operator applies
/// infix_operator!(Or, " || ", TsQuery);
///
/// // Specifying both the return types and the backend
/// infix_operator!(And, " && ", TsQuery, backend: Pg);
/// ```
///
/// ## Example usage
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use diesel::sql_types::SqlType;
/// # use diesel::expression::TypedExpressionType;
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// diesel::infix_operator!(MyEq, " = ");
///
/// use diesel::expression::AsExpression;
///
/// // Normally you would put this on a trait instead
/// fn my_eq<T, U, ST>(left: T, right: U) -> MyEq<T, U::Expression> where
///     T: Expression<SqlType = ST>,
///     U: AsExpression<ST>,
///     ST: SqlType + TypedExpressionType,
/// {
///     MyEq::new(left, right.as_expression())
/// }
///
/// let users_with_name = users.select(id).filter(my_eq(name, "Sean"));
///
/// assert_eq!(Ok(1), users_with_name.first(connection));
/// # }
/// ```
#[macro_export]
macro_rules! infix_operator {
    ($name:ident, $operator:expr) => {
        $crate::infix_operator!($name, $operator, $crate::sql_types::Bool);
    };

    ($name:ident, $operator:expr, backend: $backend:ty) => {
        $crate::infix_operator!($name, $operator, $crate::sql_types::Bool, backend: $backend);
    };

    ($name:ident, $operator:expr, $($return_ty:tt)::*) => {
        $crate::__diesel_infix_operator!(
            name = $name,
            operator = $operator,
            return_ty = NullableBasedOnArgs ($($return_ty)::*),
            backend_ty_params = (DB,),
            backend_ty = DB,
        );
    };

    ($name:ident, $operator:expr, $($return_ty:tt)::*, backend: $backend:ty) => {
        $crate::__diesel_infix_operator!(
            name = $name,
            operator = $operator,
            return_ty = NullableBasedOnArgs ($($return_ty)::*),
            backend_ty_params = (),
            backend_ty = $backend,
        );
    };

}
#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_infix_operator {
    ($name:ident, $operator:expr, ConstantNullability $($return_ty:tt)::*) => {
        $crate::__diesel_infix_operator!(
            name = $name,
            operator = $operator,
            return_ty = ($($return_ty)::*),
            backend_ty_params = (DB,),
            backend_ty = DB,
        );
    };
    ($name:ident, $operator:expr, __diesel_internal_SameResultAsInput, backend: $backend:ty) => {
        $crate::__diesel_infix_operator!(
            name = $name,
            operator = $operator,
            return_ty = (<T as $crate::expression::Expression>::SqlType),
            backend_ty_params = (),
            backend_ty = $backend,
        );
    };
    ($name:ident, $operator:expr, ConstantNullability $($return_ty:tt)::*, backend: $backend:ty) => {
        $crate::__diesel_infix_operator!(
            name = $name,
            operator = $operator,
            return_ty = ($($return_ty)::*),
            backend_ty_params = (),
            backend_ty = $backend,
        );
    };

    (
        name = $name:ident,
        operator = $operator:expr,
        return_ty = NullableBasedOnArgs ($($return_ty:tt)+),
        backend_ty_params = $backend_ty_params:tt,
        backend_ty = $backend_ty:ty,
    ) => {
        $crate::__diesel_infix_operator!(
            name = $name,
            operator = $operator,
            return_ty = (
                $crate::sql_types::is_nullable::MaybeNullable<
                    $crate::sql_types::is_nullable::IsOneNullable<
                        <T as $crate::expression::Expression>::SqlType,
                        <U as $crate::expression::Expression>::SqlType
                    >,
                    $($return_ty)+
                >
            ),
            expression_bounds = (
                $crate::sql_types::is_nullable::IsSqlTypeNullable<
                    <T as $crate::expression::Expression>::SqlType
                >: $crate::sql_types::OneIsNullable<
                    $crate::sql_types::is_nullable::IsSqlTypeNullable<
                        <U as $crate::expression::Expression>::SqlType
                    >
                >,
                $crate::sql_types::is_nullable::IsOneNullable<
                    <T as $crate::expression::Expression>::SqlType,
                    <U as $crate::expression::Expression>::SqlType
                >: $crate::sql_types::MaybeNullableType<$($return_ty)+>,
            ),
            backend_ty_params = $backend_ty_params,
            backend_ty = $backend_ty,
        );
    };

    (
        name = $name:ident,
        operator = $operator:expr,
        return_ty = ($($return_ty:tt)+),
        backend_ty_params = $backend_ty_params:tt,
        backend_ty = $backend_ty:ty,
    ) => {
        $crate::__diesel_infix_operator!(
            name = $name,
            operator = $operator,
            return_ty = ($($return_ty)+),
            expression_bounds = (),
            backend_ty_params = $backend_ty_params,
            backend_ty = $backend_ty,
        );
    };

    (
        name = $name:ident,
        operator = $operator:expr,
        return_ty = ($($return_ty:tt)+),
        expression_bounds = ($($expression_bounds:tt)*),
        backend_ty_params = $backend_ty_params:tt,
        backend_ty = $backend_ty:ty,
    ) => {
        $crate::__diesel_operator_body!(
            notation = infix,
            struct_name = $name,
            operator = $operator,
            return_ty = ($($return_ty)+),
            ty_params = (T, U,),
            field_names = (left, right,),
            backend_ty_params = $backend_ty_params,
            backend_ty = $backend_ty,
            expression_ty_params = (),
            expression_bounds = (
                T: $crate::expression::Expression,
                U: $crate::expression::Expression,
                <T as $crate::expression::Expression>::SqlType: $crate::sql_types::SqlType,
                <U as $crate::expression::Expression>::SqlType: $crate::sql_types::SqlType,
                $($expression_bounds)*
            ),
        );
    };
}

#[macro_export]
#[deprecated(since = "2.0.0", note = "use `diesel::infix_operator!` instead")]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[doc(hidden)]
macro_rules! diesel_infix_operator {
    ($($args:tt)*) => {
        $crate::infix_operator!($($args)*);
    }
}

/// Useful for libraries adding support for new SQL types. Apps should never
/// need to call this.
///
/// Similar to [`infix_operator!`], but the generated type will only take
/// a single argument rather than two. The operator SQL will be placed after
/// the single argument. See [`infix_operator!`] for example usage.
///
#[macro_export]
macro_rules! postfix_operator {
    ($name:ident, $operator:expr) => {
        $crate::postfix_operator!($name, $operator, $crate::sql_types::Bool);
    };

    ($name:ident, $operator:expr, backend: $backend:ty) => {
        $crate::postfix_operator!($name, $operator, $crate::sql_types::Bool, backend: $backend);
    };

    ($name:ident, $operator:expr, $return_ty:ty) => {
        $crate::__diesel_operator_body!(
            notation = postfix,
            struct_name = $name,
            operator = $operator,
            return_ty = ($return_ty),
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (DB,),
            backend_ty = DB,
        );
    };

    ($name:ident, $operator:expr, $return_ty:ty, backend: $backend:ty) => {
        $crate::__diesel_operator_body!(
            notation = postfix,
            struct_name = $name,
            operator = $operator,
            return_ty = ($return_ty),
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (),
            backend_ty = $backend,
        );
    };
}

#[macro_export]
#[deprecated(since = "2.0.0", note = "use `diesel::postfix_operator!` instead")]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[doc(hidden)]
macro_rules! diesel_postfix_operator {
    ($($args:tt)*) => {
        $crate::postfix_operator!($($args)*);
    }
}

/// Useful for libraries adding support for new SQL types. Apps should never
/// need to call this.
///
/// Similar to [`infix_operator!`], but the generated type will only take
/// a single argument rather than two. The operator SQL will be placed before
/// the single argument. See [`infix_operator!`] for example usage.
///
#[macro_export]
macro_rules! prefix_operator {
    ($name:ident, $operator:expr) => {
        $crate::prefix_operator!($name, $operator, $crate::sql_types::Bool);
    };

    ($name:ident, $operator:expr, backend: $backend:ty) => {
        $crate::prefix_operator!($name, $operator, $crate::sql_types::Bool, backend: $backend);
    };

    ($name:ident, $operator:expr, $return_ty:ty) => {
        $crate::__diesel_operator_body!(
            notation = prefix,
            struct_name = $name,
            operator = $operator,
            return_ty = (
                $crate::sql_types::is_nullable::MaybeNullable<
                    $crate::sql_types::is_nullable::IsSqlTypeNullable<
                        <Expr as $crate::expression::Expression>::SqlType
                    >,
                    $return_ty,
                >
            ),
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (DB,),
            backend_ty = DB,
            expression_ty_params = (),
            expression_bounds = (
                Expr: $crate::expression::Expression,
                <Expr as $crate::expression::Expression>::SqlType: $crate::sql_types::SqlType,
                $crate::sql_types::is_nullable::IsSqlTypeNullable<
                    <Expr as $crate::expression::Expression>::SqlType
                >: $crate::sql_types::MaybeNullableType<$return_ty>,
            ),
        );
    };

    ($name:ident, $operator:expr, $return_ty:ty, backend: $backend:ty) => {
        $crate::__diesel_operator_body!(
            notation = prefix,
            struct_name = $name,
            operator = $operator,
            return_ty = (
                $crate::sql_types::is_nullable::MaybeNullable<
                    $crate::sql_types::is_nullable::IsSqlTypeNullable<
                        <Expr as $crate::expression::Expression>::SqlType
                    >,
                    $return_ty,
                >
            ),
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (),
            backend_ty = $backend,
            expression_ty_params = (),
            expression_bounds = (
                Expr: $crate::expression::Expression,
                <Expr as $crate::expression::Expression>::SqlType: $crate::sql_types::SqlType,
                $crate::sql_types::is_nullable::IsSqlTypeNullable<
                    <Expr as $crate::expression::Expression>::SqlType
                >: $crate::sql_types::MaybeNullableType<$return_ty>,
            ),
        );
    };
}

#[macro_export]
#[deprecated(since = "2.0.0", note = "use `diesel::prefix_operator!` instead")]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
#[doc(hidden)]
macro_rules! diesel_prefix_operator {
    ($($args:tt)*) => {
        $crate::prefix_operator!($($args)*);
    }
}

infix_operator!(And, " AND ");
infix_operator!(Or, " OR ");
infix_operator!(Escape, " ESCAPE ");
infix_operator!(Eq, " = ");
infix_operator!(Gt, " > ");
infix_operator!(GtEq, " >= ");
infix_operator!(Lt, " < ");
infix_operator!(LtEq, " <= ");
infix_operator!(NotEq, " != ");
infix_operator!(NotLike, " NOT LIKE ");
infix_operator!(Between, " BETWEEN ");
infix_operator!(NotBetween, " NOT BETWEEN ");

postfix_operator!(IsNull, " IS NULL");
postfix_operator!(IsNotNull, " IS NOT NULL");
postfix_operator!(
    Asc,
    " ASC ",
    crate::expression::expression_types::NotSelectable
);
postfix_operator!(
    Desc,
    " DESC ",
    crate::expression::expression_types::NotSelectable
);

prefix_operator!(Not, " NOT ");

use crate::backend::{sql_dialect, Backend, SqlDialect};
use crate::expression::{TypedExpressionType, ValidGrouping};
use crate::insertable::{ColumnInsertValue, Insertable};
use crate::query_builder::{QueryFragment, QueryId, ValuesClause};
use crate::query_source::Column;
use crate::sql_types::{DieselNumericOps, SqlType};

impl<T, U> Insertable<T::Table> for Eq<T, U>
where
    T: Column,
{
    type Values = ValuesClause<ColumnInsertValue<T, U>, T::Table>;

    fn values(self) -> Self::Values {
        ValuesClause::new(ColumnInsertValue::new(self.right))
    }
}

impl<'a, T, Tab, U> Insertable<Tab> for &'a Eq<T, U>
where
    T: Copy,
    Eq<T, &'a U>: Insertable<Tab>,
{
    type Values = <Eq<T, &'a U> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        Eq::new(self.left, &self.right).values()
    }
}

/// This type represents a string concat operator
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(left, right)
)]
#[derive(Debug, Clone, Copy, QueryId, DieselNumericOps, ValidGrouping)]
pub struct Concat<L, R> {
    /// The left side expression of the operator
    pub(crate) left: L,
    /// The right side expression of the operator
    pub(crate) right: R,
}

impl<L, R> Concat<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L, R, ST> crate::expression::Expression for Concat<L, R>
where
    L: crate::expression::Expression<SqlType = ST>,
    R: crate::expression::Expression<SqlType = ST>,
    ST: SqlType + TypedExpressionType,
{
    type SqlType = ST;
}

impl_selectable_expression!(Concat<L, R>);

impl<L, R, DB> QueryFragment<DB> for Concat<L, R>
where
    DB: Backend,
    Self: QueryFragment<DB, DB::ConcatClause>,
{
    fn walk_ast<'b>(
        &'b self,
        pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::result::QueryResult<()> {
        <Self as QueryFragment<DB, DB::ConcatClause>>::walk_ast(self, pass)
    }
}

impl<L, R, DB> QueryFragment<DB, sql_dialect::concat_clause::ConcatWithPipesClause> for Concat<L, R>
where
    L: QueryFragment<DB>,
    R: QueryFragment<DB>,
    DB: Backend + SqlDialect<ConcatClause = sql_dialect::concat_clause::ConcatWithPipesClause>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::result::QueryResult<()> {
        // Since popular MySQL scalability layer Vitess does not support pipes in query parsing
        // CONCAT has been implemented separately for MySQL (see MysqlConcatClause)
        out.push_sql("(");
        self.left.walk_ast(out.reborrow())?;
        out.push_sql(" || ");
        self.right.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

// need an explicit impl here to control which types are allowed
#[derive(
    Debug,
    Clone,
    Copy,
    crate::query_builder::QueryId,
    crate::sql_types::DieselNumericOps,
    crate::expression::ValidGrouping,
)]
#[doc(hidden)]
pub struct Like<T, U> {
    pub(crate) left: T,
    pub(crate) right: U,
}

impl<T, U> Like<T, U> {
    pub(crate) fn new(left: T, right: U) -> Self {
        Like { left, right }
    }
}

impl<T, U, QS> crate::expression::SelectableExpression<QS> for Like<T, U>
where
    Like<T, U>: crate::expression::AppearsOnTable<QS>,
    T: crate::expression::SelectableExpression<QS>,
    U: crate::expression::SelectableExpression<QS>,
{
}

impl<T, U, QS> crate::expression::AppearsOnTable<QS> for Like<T, U>
where
    Like<T, U>: crate::expression::Expression,
    T: crate::expression::AppearsOnTable<QS>,
    U: crate::expression::AppearsOnTable<QS>,
{
}

impl<T, U> crate::expression::Expression for Like<T, U>
where
    T: crate::expression::Expression,
    U: crate::expression::Expression,
    <T as crate::expression::Expression>::SqlType: crate::sql_types::SqlType,
    <U as crate::expression::Expression>::SqlType: crate::sql_types::SqlType,
    crate::sql_types::is_nullable::IsSqlTypeNullable<<T as crate::expression::Expression>::SqlType>:
        crate::sql_types::OneIsNullable<
            crate::sql_types::is_nullable::IsSqlTypeNullable<
                <U as crate::expression::Expression>::SqlType,
            >,
        >,
    crate::sql_types::is_nullable::IsOneNullable<
        <T as crate::expression::Expression>::SqlType,
        <U as crate::expression::Expression>::SqlType,
    >: crate::sql_types::MaybeNullableType<crate::sql_types::Bool>,
{
    type SqlType = crate::sql_types::is_nullable::MaybeNullable<
        crate::sql_types::is_nullable::IsOneNullable<
            <T as crate::expression::Expression>::SqlType,
            <U as crate::expression::Expression>::SqlType,
        >,
        crate::sql_types::Bool,
    >;
}

impl<T, U, DB> crate::query_builder::QueryFragment<DB> for Like<T, U>
where
    T: crate::query_builder::QueryFragment<DB> + crate::Expression,
    U: crate::query_builder::QueryFragment<DB>,
    DB: crate::backend::Backend,
    DB: LikeIsAllowedForType<T::SqlType>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> crate::result::QueryResult<()> {
        (self.left.walk_ast(out.reborrow())?);
        (out.push_sql(" LIKE "));
        (self.right.walk_ast(out.reborrow())?);
        Ok(())
    }
}

impl<S, T, U> crate::internal::operators_macro::FieldAliasMapper<S> for Like<T, U>
where
    S: crate::query_source::AliasSource,
    T: crate::internal::operators_macro::FieldAliasMapper<S>,
    U: crate::internal::operators_macro::FieldAliasMapper<S>,
{
    type Out = Like<
        <T as crate::internal::operators_macro::FieldAliasMapper<S>>::Out,
        <U as crate::internal::operators_macro::FieldAliasMapper<S>>::Out,
    >;
    fn map(self, alias: &crate::query_source::Alias<S>) -> Self::Out {
        Like {
            left: self.left.map(alias),
            right: self.right.map(alias),
        }
    }
}

#[diagnostic::on_unimplemented(
    message = "Cannot use the `LIKE` operator with expressions of the type `{ST}` for the backend `{Self}`",
    note = "Expressions of the type `diesel::sql_types::Text` and `diesel::sql_types::Nullable<Text>` are \n\
            allowed for all backends"
)]
#[cfg_attr(
    feature = "postgres_backend",
    diagnostic::on_unimplemented(
        note = "Expressions of the type `diesel::sql_types::Binary` and `diesel::sql_types::Nullable<Binary>` are \n\
            allowed for the PostgreSQL backend"
    )
)]
pub trait LikeIsAllowedForType<ST>: Backend {}

impl<DB> LikeIsAllowedForType<crate::sql_types::Text> for DB where DB: Backend {}

impl<T, DB> LikeIsAllowedForType<crate::sql_types::Nullable<T>> for DB where
    DB: Backend + LikeIsAllowedForType<T>
{
}
