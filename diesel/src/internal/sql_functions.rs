#[doc(hidden)]
pub use crate::expression::functions::aggregate_expressions::{
    FunctionFragment, IsAggregateFunction, IsWindowFunction, Order, OverClause,
    WindowFunctionFragment,
};

#[macro_export]
#[doc(hidden)]
#[cfg(feature = "__sqlite-shared")]
macro_rules! expand_sqlite_function {
    ([], $($tt:tt)*) => {$($tt)*};
    ([$t:ident, $($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!($t, [$($ts,)*], $($tt)*);
    };
    // explicitly match allowed types
    (Integer, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (BigInt, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Binary, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Bool, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Date, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Double, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Float, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Numeric, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (SmallInt, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Text, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Time, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    (Timestamp, [$($ts:ident,)*], $($tt:tt)*) => {
        expand_sqlite_function!([$($ts,)*], $($tt)*);
    };
    // ignore any other type
    ($t:ident, [$($ts:ident,)*], $($tt:tt)*) => {};
}

#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "__sqlite-shared"))]
macro_rules! expand_sqlite_function {
    ([$($ts:ty,)*], $($tt:tt)*) => {};
}

#[doc(hidden)]
pub use crate::{expand_mysql, expand_pg, expand_sqlite};
#[doc(hidden)]
pub use expand_sqlite_function;
