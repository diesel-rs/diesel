#[macro_export]
macro_rules! operator_allowed {
    ($tpe: ty, $op: ident, $fn_name: ident) => {
        impl<Rhs> ::std::ops::$op<Rhs> for $tpe where
            Rhs: $crate::expression::AsExpression<
                <<$tpe as $crate::Expression>::SqlType as $crate::types::ops::$op>::Rhs
            >,
        {
            type Output = $crate::expression::ops::$op<Self, Rhs::Expression>;

            fn $fn_name(self, rhs: Rhs) -> Self::Output {
                $crate::expression::ops::$op::new(self, rhs.as_expression())
            }
        }
    }
}

#[macro_export]
macro_rules! numeric_expr {
    ($tpe: ty) => {
        operator_allowed!($tpe, Add, add);
        operator_allowed!($tpe, Sub, sub);
        operator_allowed!($tpe, Div, div);
        operator_allowed!($tpe, Mul, mul);
    }
}

macro_rules! generic_numeric_expr_inner {
    ($tpe: ident, ($($param: ident),*), $op: ident, $fn_name: ident) => {
        impl<Rhs, $($param),*> ::std::ops::$op<Rhs> for $tpe<$($param),*> where
            $tpe<$($param),*>: $crate::expression::Expression,
            <$tpe<$($param),*> as $crate::Expression>::SqlType: $crate::types::ops::$op,
            Rhs: $crate::expression::AsExpression<
                <<$tpe<$($param),*> as $crate::Expression>::SqlType as $crate::types::ops::$op>::Rhs,
            >,
        {
            type Output = $crate::expression::ops::$op<Self, Rhs::Expression>;

            fn $fn_name(self, rhs: Rhs) -> Self::Output {
                $crate::expression::ops::$op::new(self, rhs.as_expression())
            }
        }
    }
}

#[macro_export]
macro_rules! generic_numeric_expr {
    ($tpe: ident, $($param: ident),*) => {
        generic_numeric_expr_inner!($tpe, ($($param),*), Add, add);
        generic_numeric_expr_inner!($tpe, ($($param),*), Sub, sub);
        generic_numeric_expr_inner!($tpe, ($($param),*), Div, div);
        generic_numeric_expr_inner!($tpe, ($($param),*), Mul, mul);
    }
}

mod numeric;

pub use self::numeric::{Add, Sub, Mul, Div};
