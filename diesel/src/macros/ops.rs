#[macro_export]
/// Implements the Rust operator for a given type. If you create a new SQL
/// function, which returns a type that you'd like to use an operator on, you
/// should invoke this macro. Unfortunately, Rust disallows us from
/// automatically implementing `Add` and other traits from `std::ops`, under its
/// orphan rules.
macro_rules! operator_allowed {
    ($tpe:ty, $op:ident, $fn_name:ident) => {
        impl<Rhs> ::std::ops::$op<Rhs> for $tpe
        where
            Rhs: $crate::expression::AsExpression<
                <<$tpe as $crate::Expression>::SqlType as $crate::sql_types::ops::$op>::Rhs,
            >,
        {
            type Output = $crate::expression::ops::$op<Self, Rhs::Expression>;

            fn $fn_name(self, rhs: Rhs) -> Self::Output {
                $crate::expression::ops::$op::new(self, rhs.as_expression())
            }
        }
    };
}

#[macro_export]
/// Indicates that an expression allows all numeric operators. If you create new
/// SQL functions that return a numeric type, you should invoke this macro that
/// type. Unfortunately, Rust disallows us from automatically implementing `Add`
/// for types which implement `Expression`, under its orphan rules.
macro_rules! numeric_expr {
    ($tpe:ty) => {
        operator_allowed!($tpe, Add, add);
        operator_allowed!($tpe, Sub, sub);
        operator_allowed!($tpe, Div, div);
        operator_allowed!($tpe, Mul, mul);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_generate_ops_impls_if_numeric {
    ($column_name:ident, Nullable<$($inner:tt)::*>) => { __diesel_generate_ops_impls_if_numeric!($column_name, $($inner)::*); };

    ($column_name:ident, SmallInt) => { numeric_expr!($column_name); };
    ($column_name:ident, Int2) => { numeric_expr!($column_name); };
    ($column_name:ident, Smallint) => { numeric_expr!($column_name); };
    ($column_name:ident, SmallSerial) => { numeric_expr!($column_name); };

    ($column_name:ident, Integer) => { numeric_expr!($column_name); };
    ($column_name:ident, Int4) => { numeric_expr!($column_name); };
    ($column_name:ident, Serial) => { numeric_expr!($column_name); };

    ($column_name:ident, BigInt) => { numeric_expr!($column_name); };
    ($column_name:ident, Int8) => { numeric_expr!($column_name); };
    ($column_name:ident, Bigint) => { numeric_expr!($column_name); };
    ($column_name:ident, BigSerial) => { numeric_expr!($column_name); };

    ($column_name:ident, Float) => { numeric_expr!($column_name); };
    ($column_name:ident, Float4) => { numeric_expr!($column_name); };

    ($column_name:ident, Double) => { numeric_expr!($column_name); };
    ($column_name:ident, Float8) => { numeric_expr!($column_name); };

    ($column_name:ident, Numeric) => { numeric_expr!($column_name); };

    ($column_name:ident, $non_numeric_type:ty) => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! date_time_expr {
    ($tpe:ty) => {
        operator_allowed!($tpe, Add, add);
        operator_allowed!($tpe, Sub, sub);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __diesel_generate_ops_impls_if_date_time {
    ($column_name:ident, Nullable<$($inner:tt)::*>) => { __diesel_generate_ops_impls_if_date_time!($column_name, $($inner)::*); };
    ($column_name:ident, Time) => { date_time_expr!($column_name); };
    ($column_name:ident, Date) => { date_time_expr!($column_name); };
    ($column_name:ident, Timestamp) => { date_time_expr!($column_name); };
    ($column_name:ident, Timestamptz) => { date_time_expr!($column_name); };
    ($column_name:ident, $non_date_time_type:ty) => {};
}
