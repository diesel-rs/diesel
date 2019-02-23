#![cfg_attr(rustfmt, rustfmt_skip)] // https://github.com/rust-lang-nursery/rustfmt/issues/2755

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_operator_body {
    (
        notation = $notation:ident,
        struct_name = $name:ident,
        operator = $operator:expr,
        return_ty = ReturnBasedOnArgs,
        ty_params = ($($ty_param:ident,)+),
        field_names = $field_names:tt,
        backend_ty_params = $backend_ty_params:tt,
        backend_ty = $backend_ty:ty,
    ) => {
        __diesel_operator_body! {
            notation = $notation,
            struct_name = $name,
            operator = $operator,
            return_ty = ST,
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
        return_ty = $return_ty:ty,
        ty_params = ($($ty_param:ident,)+),
        field_names = $field_names:tt,
        backend_ty_params = $backend_ty_params:tt,
        backend_ty = $backend_ty:ty,
    ) => {
        __diesel_operator_body! {
            notation = $notation,
            struct_name = $name,
            operator = $operator,
            return_ty = $return_ty,
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
        return_ty = $return_ty:ty,
        ty_params = ($($ty_param:ident,)+),
        field_names = ($($field_name:ident,)+),
        backend_ty_params = ($($backend_ty_param:ident,)*),
        backend_ty = $backend_ty:ty,
        expression_ty_params = ($($expression_ty_params:ident,)*),
        expression_bounds = ($($expression_bounds:tt)*),
    ) => {
        #[derive(Debug, Clone, Copy, QueryId, DieselNumericOps, NonAggregate)]
        #[doc(hidden)]
        pub struct $name<$($ty_param,)+> {
            $(pub(crate) $field_name: $ty_param,)+
        }

        impl<$($ty_param,)+> $name<$($ty_param,)+> {
            pub fn new($($field_name: $ty_param,)+) -> Self {
                $name { $($field_name,)+ }
            }
        }

        impl_selectable_expression!($name<$($ty_param),+>);

        impl<$($ty_param,)+ $($expression_ty_params,)*> $crate::expression::Expression for $name<$($ty_param,)+> where
            $($expression_bounds)*
        {
            type SqlType = $return_ty;
        }

        impl<$($ty_param,)+ $($backend_ty_param,)*> $crate::query_builder::QueryFragment<$backend_ty>
            for $name<$($ty_param,)+> where
                $($ty_param: $crate::query_builder::QueryFragment<$backend_ty>,)+
                $($backend_ty_param: $crate::backend::Backend,)*
        {
            fn walk_ast(&self, mut out: $crate::query_builder::AstPass<$backend_ty>) -> $crate::result::QueryResult<()> {
                __diesel_operator_to_sql!(
                    notation = $notation,
                    operator_expr = out.push_sql($operator),
                    field_exprs = ($(self.$field_name.walk_ast(out.reborrow())?),+),
                );
                Ok(())
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
/// diesel_infix_operator!(Matches, " @@ ");
///
/// // Queries which try to execute `Contains` on a backend other than Pg
/// // will fail to compile
/// diesel_infix_operator!(Contains, " @> ", backend: Pg);
///
/// // The type of `Concat` will be `TsVector` rather than Bool
/// diesel_infix_operator!(Concat, " || ", TsVector);
///
/// // It is perfectly fine to have multiple operators with the same SQL.
/// // Diesel will ensure that the queries are always unambiguous in which
/// // operator applies
/// diesel_infix_operator!(Or, " || ", TsQuery);
///
/// // Specifying both the return types and the backend
/// diesel_infix_operator!(And, " && ", TsQuery, backend: Pg);
/// ```
///
/// ## Example usage
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// #
/// # fn main() {
/// #     use schema::users::dsl::*;
/// #     let connection = establish_connection();
/// diesel_infix_operator!(MyEq, " = ");
///
/// use diesel::expression::AsExpression;
///
/// // Normally you would put this on a trait instead
/// fn my_eq<T, U>(left: T, right: U) -> MyEq<T, U::Expression> where
///     T: Expression,
///     U: AsExpression<T::SqlType>,
/// {
///     MyEq::new(left, right.as_expression())
/// }
///
/// let users_with_name = users.select(id).filter(my_eq(name, "Sean"));
///
/// assert_eq!(Ok(1), users_with_name.first(&connection));
/// # }
/// ```
#[macro_export]
macro_rules! diesel_infix_operator {
    ($name:ident, $operator:expr) => {
        diesel_infix_operator!($name, $operator, $crate::sql_types::Bool);
    };

    ($name:ident, $operator:expr, backend: $backend:ty) => {
        diesel_infix_operator!($name, $operator, $crate::sql_types::Bool, backend: $backend);
    };

    ($name:ident, $operator:expr, $($return_ty:tt)::*) => {
        __diesel_operator_body!(
            notation = infix,
            struct_name = $name,
            operator = $operator,
            return_ty = $($return_ty)::*,
            ty_params = (T, U,),
            field_names = (left, right,),
            backend_ty_params = (DB,),
            backend_ty = DB,
        );
    };

    ($name:ident, $operator:expr, $return_ty:ty, backend: $backend:ty) => {
        __diesel_operator_body!(
            notation = infix,
            struct_name = $name,
            operator = $operator,
            return_ty = $return_ty,
            ty_params = (T, U,),
            field_names = (left, right,),
            backend_ty_params = (),
            backend_ty = $backend,
        );
    };
}

/// Useful for libraries adding support for new SQL types. Apps should never
/// need to call this.
///
/// Similar to [`diesel_infix_operator!`], but the generated type will only take
/// a single argument rather than two. The operator SQL will be placed after
/// the single argument. See [`diesel_infix_operator!`] for example usage.
///
/// [`diesel_infix_operator!`]: macro.diesel_infix_operator.html
#[macro_export]
macro_rules! diesel_postfix_operator {
    ($name:ident, $operator:expr) => {
        diesel_postfix_operator!($name, $operator, $crate::sql_types::Bool);
    };

    ($name:ident, $operator:expr, backend: $backend:ty) => {
        diesel_postfix_operator!($name, $operator, $crate::sql_types::Bool, backend: $backend);
    };

    ($name:ident, $operator:expr, $return_ty:ty) => {
        __diesel_operator_body!(
            notation = postfix,
            struct_name = $name,
            operator = $operator,
            return_ty = $return_ty,
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (DB,),
            backend_ty = DB,
        );
    };

    ($name:ident, $operator:expr, $return_ty:ty, backend: $backend:ty) => {
        __diesel_operator_body!(
            notation = postfix,
            struct_name = $name,
            operator = $operator,
            return_ty = $return_ty,
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (),
            backend_ty = $backend,
        );
    };
}

/// Useful for libraries adding support for new SQL types. Apps should never
/// need to call this.
///
/// Similar to [`diesel_infix_operator!`], but the generated type will only take
/// a single argument rather than two. The operator SQL will be placed before
/// the single argument. See [`diesel_infix_operator!`] for example usage.
///
/// [`diesel_infix_operator!`]: macro.diesel_infix_operator.html
#[macro_export]
macro_rules! diesel_prefix_operator {
    ($name:ident, $operator:expr) => {
        diesel_prefix_operator!($name, $operator, $crate::sql_types::Bool);
    };

    ($name:ident, $operator:expr, backend: $backend:ty) => {
        diesel_prefix_operator!($name, $operator, $crate::sql_types::Bool, backend: $backend);
    };

    ($name:ident, $operator:expr, $return_ty:ty) => {
        __diesel_operator_body!(
            notation = prefix,
            struct_name = $name,
            operator = $operator,
            return_ty = $return_ty,
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (DB,),
            backend_ty = DB,
        );
    };

    ($name:ident, $operator:expr, $return_ty:ty, backend: $backend:ty) => {
        __diesel_operator_body!(
            notation = prefix,
            struct_name = $name,
            operator = $operator,
            return_ty = $return_ty,
            ty_params = (Expr,),
            field_names = (expr,),
            backend_ty_params = (),
            backend_ty = $backend,
        );
    };
}

diesel_infix_operator!(Concat, " || ", ReturnBasedOnArgs);
diesel_infix_operator!(And, " AND ");
diesel_infix_operator!(Between, " BETWEEN ");
diesel_infix_operator!(Escape, " ESCAPE ");
diesel_infix_operator!(Eq, " = ");
diesel_infix_operator!(Gt, " > ");
diesel_infix_operator!(GtEq, " >= ");
diesel_infix_operator!(Like, " LIKE ");
diesel_infix_operator!(Lt, " < ");
diesel_infix_operator!(LtEq, " <= ");
diesel_infix_operator!(NotBetween, " NOT BETWEEN ");
diesel_infix_operator!(NotEq, " != ");
diesel_infix_operator!(NotLike, " NOT LIKE ");
diesel_infix_operator!(Or, " OR ");

diesel_postfix_operator!(IsNull, " IS NULL");
diesel_postfix_operator!(IsNotNull, " IS NOT NULL");
diesel_postfix_operator!(Asc, " ASC", ());
diesel_postfix_operator!(Desc, " DESC", ());

diesel_prefix_operator!(Not, "NOT ");

use insertable::{ColumnInsertValue, Insertable};
use query_builder::ValuesClause;
use query_source::Column;

impl<T, U> Insertable<T::Table> for Eq<T, U>
where
    T: Column,
{
    type Values = ValuesClause<ColumnInsertValue<T, U>, T::Table>;

    fn values(self) -> Self::Values {
        ValuesClause::new(ColumnInsertValue::Expression(self.left, self.right))
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
