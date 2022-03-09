macro_rules! generic_numeric_expr_inner {
    ($tpe: ident, ($($param: ident),*), $op: ident, $fn_name: ident) => {
        impl<Rhs, $($param),*> ::std::ops::$op<Rhs> for $tpe<$($param),*> where
            $tpe<$($param),*>: $crate::expression::Expression,
            <$tpe<$($param),*> as $crate::Expression>::SqlType: $crate::sql_types::SqlType + $crate::sql_types::ops::$op,
            <<$tpe<$($param),*> as $crate::Expression>::SqlType as $crate::sql_types::ops::$op>::Rhs: $crate::expression::TypedExpressionType,
            Rhs: $crate::expression::AsExpression<
                <<$tpe<$($param),*> as $crate::Expression>::SqlType as $crate::sql_types::ops::$op>::Rhs,
            >,
        {
            type Output = $crate::expression::ops::$op<Self, Rhs::Expression>;

            fn $fn_name(self, rhs: Rhs) -> Self::Output {
                $crate::expression::ops::$op::new(self, rhs.as_expression())
            }
        }
    }
}

macro_rules! generic_numeric_expr {
    ($tpe: ident, $($param: ident),*) => {
        generic_numeric_expr_inner!($tpe, ($($param),*), Add, add);
        generic_numeric_expr_inner!($tpe, ($($param),*), Sub, sub);
        generic_numeric_expr_inner!($tpe, ($($param),*), Div, div);
        generic_numeric_expr_inner!($tpe, ($($param),*), Mul, mul);
    }
}

pub(crate) mod numeric;

pub(crate) use self::numeric::{Add, Div, Mul, Sub};
