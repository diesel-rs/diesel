use types::{self, NativeSqlType};

pub trait NumericSqlType: NativeSqlType {
}

macro_rules! numeric_type {
    ($($tpe: ident),*) => {
        $(impl NumericSqlType for types::$tpe {
        })*
    }
}

numeric_type!(SmallInt, Integer, BigInt, Float, Double);

#[macro_export]
macro_rules! numeric_expr_inner {
    ($tpe: ty, $op: ident, $fn_name: ident) => {
        impl<Rhs> ::std::ops::$op<Rhs> for $tpe where
            Rhs: $crate::expression::AsExpression<<$tpe as $crate::Expression>::SqlType>,
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
        numeric_expr_inner!($tpe, Add, add);
        numeric_expr_inner!($tpe, Sub, sub);
        numeric_expr_inner!($tpe, Div, div);
        numeric_expr_inner!($tpe, Mul, mul);
    }
}

macro_rules! generic_numeric_expr_inner {
    ($tpe: ident, ($($param: ident),*), $op: ident, $fn_name: ident) => {
        impl<Rhs, $($param),*> ::std::ops::$op<Rhs> for $tpe<$($param),*> where
            $tpe<$($param),*>: $crate::expression::Expression,
            Rhs: $crate::expression::AsExpression<<$tpe<$($param),*> as $crate::Expression>::SqlType>,
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
