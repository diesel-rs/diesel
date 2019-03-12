#[macro_export]
/// Declare a sql function for use in your code.
///
/// Diesel only provides support for a very small number of SQL functions.
/// This macro enables you to add additional functions from the SQL standard,
/// as well as any custom functions your application might have.
///
/// The syntax for this macro is very similar to that of a normal Rust function,
/// except the argument and return types will be the SQL types being used.
/// Typically these types will come from [`diesel::sql_types`].
///
/// This macro will generate two items. A function with the name that you've
/// given, and a module with a helper type representing the return type of your
/// function. For example, this invocation:
///
/// ```ignore
/// sql_function!(fn lower(x: Text) -> Text);
/// ```
///
/// will generate this code:
///
/// ```ignore
/// pub fn lower<X>(x: X) -> lower::HelperType<X> {
///     ...
/// }
///
/// pub(crate) mod lower {
///     pub type HelperType<X> = ...;
/// }
/// ```
///
/// If you are using this macro for part of a library, where the function is
/// part of your public API, it is highly recommended that you re-export this
/// helper type with the same name as your function. This is the standard
/// structure:
///
/// ```ignore
/// pub mod functions {
///     use super::types::*;
///     use diesel::sql_types::*;
///
///     sql_function! {
///         /// Represents the Pg `LENGTH` function used with `tsvector`s.
///         fn length(x: TsVector) -> Integer;
///     }
/// }
///
/// pub mod helper_types {
///     /// The return type of `length(expr)`
///     pub type Length<Expr> = functions::length::HelperType<Expr>;
/// }
///
/// pub mod dsl {
///     pub use functions::*;
///     pub use helper_types::*;
/// }
/// ```
///
/// Most attributes given to this macro will be put on the generated function
/// (including doc comments).
///
/// # Adding Doc Comments
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// use diesel::sql_types::Text;
///
/// sql_function! {
///     /// Represents the `canon_crate_name` SQL function, created in
///     /// migration ....
///     fn canon_crate_name(a: Text) -> Text;
/// }
///
/// # fn main() {
/// # use self::crates::dsl::*;
/// let target_name = "diesel";
/// crates.filter(canon_crate_name(name).eq(canon_crate_name(target_name)));
/// // This will generate the following SQL
/// // SELECT * FROM crates WHERE canon_crate_name(crates.name) = canon_crate_name($1)
/// # }
/// ```
///
/// # Special Attributes
///
/// There are a handful of special attributes that Diesel will recognize. They
/// are:
///
/// - `#[aggregate]`
///   - Indicates that this is an aggregate function, and that `NonAggregate`
///     should not be implemented.
/// - `#[sql_name="name"]`
///   - The SQL to be generated is different than the Rust name of the function.
///     This can be used to represent functions which can take many argument
///     types, or to capitalize function names.
///
/// Functions can also be generic. Take the definition of `sum` for an example
/// of all of this:
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// use diesel::sql_types::Foldable;
///
/// sql_function! {
///     #[aggregate]
///     #[sql_name = "SUM"]
///     fn sum<ST: Foldable>(expr: ST) -> ST::Sum;
/// }
///
/// # fn main() {
/// # use self::crates::dsl::*;
/// crates.select(sum(id));
/// # }
/// ```
///
/// # Use with SQLite
///
/// On most backends, the implementation of the function is defined in a
/// migration using `CREATE FUNCTION`. On SQLite, the function is implemented in
/// Rust instead. You must call `register_impl` or
/// `register_nondeterministic_impl` with every connection before you can use
/// the function.
///
/// These functions will only be generated if the `sqlite` feature is enabled,
/// and the function is not generic. Generic functions and variadic functions
/// are not supported on SQLite.
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # use diesel::*;
/// #
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {
/// # }
/// #
/// use diesel::sql_types::{Integer, Double};
/// sql_function!(fn add_mul(x: Integer, y: Integer, z: Double) -> Double);
///
/// # #[cfg(feature = "sqlite")]
/// # fn run_test() -> Result<(), Box<::std::error::Error>> {
/// let connection = SqliteConnection::establish(":memory:")?;
///
/// add_mul::register_impl(&connection, |x: i32, y: i32, z: f64| {
///     (x + y) as f64 * z
/// })?;
///
/// let result = select(add_mul(1, 2, 1.5))
///     .get_result::<f64>(&connection)?;
/// assert_eq!(4.5, result);
/// #     Ok(())
/// # }
/// ```
macro_rules! sql_function {
    ($($args:tt)*) => {
        sql_function_proc! { $($args)* }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! no_arg_sql_function_body_except_to_sql {
    ($type_name:ident, $return_type:ty, $docs:expr) => {
        #[allow(non_camel_case_types)]
        #[doc=$docs]
        #[derive(Debug, Clone, Copy, QueryId, NonAggregate)]
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
macro_rules! no_arg_sql_function_body {
    ($type_name:ident, $return_type:ty, $docs:expr, $($constraint:ident)::+) => {
        no_arg_sql_function_body_except_to_sql!($type_name, $return_type, $docs);

        impl<DB> $crate::query_builder::QueryFragment<DB> for $type_name where
            DB: $crate::backend::Backend + $($constraint)::+,
        {
            fn walk_ast(&self, mut out: $crate::query_builder::AstPass<DB>) -> $crate::result::QueryResult<()> {
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
            fn walk_ast(&self, mut out: $crate::query_builder::AstPass<DB>) -> $crate::result::QueryResult<()> {
                out.push_sql(concat!(stringify!($type_name), "()"));
                Ok(())
            }
        }
    };
}

#[macro_export]
/// Declare a 0 argument SQL function for use in your code. This will generate a
/// unit struct, which is an expression representing calling this function. See
/// [`now`](expression/dsl/struct.now.html) for example output. `now` was
/// generated using:
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # pub use diesel::*;
/// no_arg_sql_function!(now, sql_types::Timestamp, "Represents the SQL NOW() function");
/// # fn main() {}
/// ```
///
/// You can optionally pass the name of a trait, as a constraint for backends which support the
/// function.
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

pub mod aggregate_folding;
pub mod aggregate_ordering;
pub mod date_and_time;
pub mod helper_types;
