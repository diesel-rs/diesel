use types::{self, NativeSqlType};

pub trait NumericSqlType: NativeSqlType {
}

macro_rules! numeric_type {
    ($($tpe: ident),*) => {
        $(impl NumericSqlType for types::$tpe {
        })*
    }
}

numeric_type!(SmallSerial, Serial, BigSerial, SmallInt, Integer, BigInt, Float, Double);

#[macro_export]
macro_rules! addable_expr {
    ($tpe: ty) => {
        impl<Rhs> ::std::ops::Add<Rhs> for $tpe where
            Rhs: $crate::expression::AsExpression<<$tpe as $crate::Expression>::SqlType>,
        {
            type Output = $crate::expression::ops::Add<Self, Rhs::Expression>;

            fn add(self, rhs: Rhs) -> Self::Output {
                $crate::expression::ops::Add::new(self, rhs.as_expression())
            }
        }
    }
}

#[macro_export]
macro_rules! generic_addable_expr {
    ($tpe: ident, $($param: ident),*) => {
        impl<Rhs, $($param),*> ::std::ops::Add<Rhs> for $tpe<$($param),*> where
            $tpe<$($param),*>: $crate::expression::Expression,
            Rhs: $crate::expression::AsExpression<<$tpe<$($param),*> as $crate::Expression>::SqlType>,
        {
            type Output = $crate::expression::ops::Add<Self, Rhs::Expression>;

            fn add(self, rhs: Rhs) -> Self::Output {
                $crate::expression::ops::Add::new(self, rhs.as_expression())
            }
        }
    }
}

mod add;

pub use self::add::Add;
