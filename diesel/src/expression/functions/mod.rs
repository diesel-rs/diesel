//! Helper macros to define custom sql functions

#[doc(inline)]
pub use diesel_derives::define_sql_function;

#[doc(inline)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub use diesel_derives::sql_function_proc as sql_function;

#[macro_export]
#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
macro_rules! no_arg_sql_function_body_except_to_sql {
    ($type_name:ident, $return_type:ty, $docs:expr) => {
        #[allow(non_camel_case_types)]
        #[doc=$docs]
        #[derive(
            Debug, Clone, Copy, $crate::query_builder::QueryId, $crate::expression::ValidGrouping,
        )]
        pub struct $type_name;

        impl $crate::expression::Expression for $type_name {
            type SqlType = $return_type;
        }

        impl<QS> $crate::expression::SelectableExpression<QS> for $type_name {}

        impl<QS> $crate::expression::AppearsOnTable<QS> for $type_name {}
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
macro_rules! no_arg_sql_function_body {
    ($type_name:ident, $return_type:ty, $docs:expr, $($constraint:ident)::+) => {
        no_arg_sql_function_body_except_to_sql!($type_name, $return_type, $docs);

        impl<DB> $crate::query_builder::QueryFragment<DB> for $type_name where
            DB: $crate::backend::Backend + $($constraint)::+,
        {
            fn walk_ast<'b>(&'b self, mut out: $crate::query_builder::AstPass<'_, 'b, DB>) -> $crate::result::QueryResult<()>
            {
                out.push_sql(concat!(stringify!($type_name), "()"));
                Ok(())
            }
        }
    };

    ($type_name:ident, $return_type:ty, $docs:expr) => {
        no_arg_sql_function_body_except_to_sql!($type_name, $return_type, $docs);

        impl<DB> $crate::query_builder::QueryFragment<DB> for $type_name where
            DB: $crate::backend::Backend,
        {
            fn walk_ast<'b>(&'b self, mut out: $crate::query_builder::AstPass<'_, 'b, DB>) -> $crate::result::QueryResult<()> {
                out.push_sql(concat!(stringify!($type_name), "()"));
                Ok(())
            }
        }
    };
}

#[macro_export]
/// Declare a 0 argument SQL function for use in your code. This will generate a
/// unit struct, which is an expression representing calling this function. See
/// [`now`](crate::expression::dsl::now) for example output. `now` was
/// generated using:
///
/// ```no_run
/// # pub use diesel::*;
/// no_arg_sql_function!(now, sql_types::Timestamp, "Represents the SQL NOW() function");
/// # fn main() {}
/// ```
///
/// You can optionally pass the name of a trait, as a constraint for backends which support the
/// function.
#[deprecated(
    since = "2.0.0",
    note = "Use `define_sql_function!` instead. See `CHANGELOG.md` for migration instructions"
)]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
macro_rules! no_arg_sql_function {
    ($type_name:ident, $return_type:ty) => {
        no_arg_sql_function!($type_name, $return_type, "");
    };

    ($type_name:ident, $return_type:ty, $docs:expr) => {
        no_arg_sql_function_body!($type_name, $return_type, $docs);
    };

    ($type_name:ident, $return_type:ty, $docs:expr, $($constraint:ident)::+) => {
        no_arg_sql_function_body!($type_name, $return_type, $docs, $($constraint)::+);
    };
}

pub(crate) mod aggregate_folding;
pub(crate) mod aggregate_ordering;
pub(crate) mod date_and_time;
pub(crate) mod helper_types;
