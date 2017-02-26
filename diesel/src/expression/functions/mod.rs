#[macro_export]
#[doc(hidden)]
macro_rules! sql_function_body {
    ($fn_name:ident, $struct_name:ident, ($($arg_name:ident: $arg_type:ty),*) -> $return_type:ty,
    $docs: expr) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy)]
        #[doc(hidden)]
        pub struct $struct_name<$($arg_name),*> {
            $($arg_name: $arg_name),*
        }

        #[allow(non_camel_case_types)]
        pub type $fn_name<$($arg_name),*> = $struct_name<$(
            <$arg_name as $crate::expression::AsExpression<$arg_type>>::Expression
        ),*>;

        #[allow(non_camel_case_types)]
        #[doc=$docs]
        pub fn $fn_name<$($arg_name),*>($($arg_name: $arg_name),*)
            -> $fn_name<$($arg_name),*>
            where $($arg_name: $crate::expression::AsExpression<$arg_type>),+
        {
            $struct_name {
                $($arg_name: $arg_name.as_expression()),+
            }
        }

        #[allow(non_camel_case_types)]
        impl<$($arg_name),*> $crate::expression::Expression for $struct_name<$($arg_name),*> where
            for <'a> ($(&'a $arg_name),*): $crate::expression::Expression,
        {
            type SqlType = $return_type;
        }

        #[allow(non_camel_case_types)]
        impl<$($arg_name),*, DB> $crate::query_builder::QueryFragment<DB> for $struct_name<$($arg_name),*> where
            DB: $crate::backend::Backend,
            for <'a> ($(&'a $arg_name),*): $crate::query_builder::QueryFragment<DB>,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                use $crate::query_builder::QueryBuilder;
                out.push_sql(concat!(stringify!($fn_name), "("));
                try!($crate::query_builder::QueryFragment::to_sql(
                        &($(&self.$arg_name),*), out));
                out.push_sql(")");
                Ok(())
            }

            fn collect_binds(&self, out: &mut DB::BindCollector) -> $crate::result::QueryResult<()> {
                try!($crate::query_builder::QueryFragment::collect_binds(
                        &($(&self.$arg_name),*), out));
                Ok(())
            }

            fn is_safe_to_cache_prepared(&self) -> bool {
                ($(&self.$arg_name),*).is_safe_to_cache_prepared()
            }
        }

        impl_query_id!($struct_name<$($arg_name),+>);

        #[allow(non_camel_case_types)]
        impl<$($arg_name),*, QS> $crate::expression::SelectableExpression<QS> for $struct_name<$($arg_name),*> where
            $($arg_name: $crate::expression::SelectableExpression<
              QS,
              SqlTypeForSelect = <$arg_name as $crate::expression::Expression>::SqlType,
            >,)*
            $struct_name<$($arg_name),*>: $crate::expression::AppearsOnTable<QS>,
        {
            type SqlTypeForSelect = Self::SqlType;
        }

        #[allow(non_camel_case_types)]
        impl<$($arg_name),*, QS> $crate::expression::AppearsOnTable<QS> for $struct_name<$($arg_name),*> where
            $($arg_name: $crate::expression::AppearsOnTable<QS>,)*
            $struct_name<$($arg_name),*>: $crate::expression::Expression,
        {
        }

        #[allow(non_camel_case_types)]
        impl<$($arg_name),*> $crate::expression::NonAggregate for $struct_name<$($arg_name),*> where
            $($arg_name: $crate::expression::NonAggregate,)*
            $struct_name<$($arg_name),*>: $crate::expression::Expression,
        {
        }
    }
}

#[macro_export]
/// Declare a sql function for use in your code. Useful if you have your own SQL functions that
/// you'd like to use. You can optionally provide a doc string as well. `$struct_name` should just
/// be any unique name. You will not need to reference it in your code, but it is required due to
/// the fact that [`concat_idents!` is
/// useless](https://github.com/rust-lang/rust/issues/29599#issuecomment-153927167).
///
/// This will generate a rust function with the same name to construct the expression, and a helper
/// type which represents the return type of that function. The function will automatically convert
/// its arguments to expressions.
///
/// # Example
///
/// ```no_run
/// # #[macro_use] extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// sql_function!(canon_crate_name, canon_crate_name_t, (a: types::VarChar) -> types::VarChar);
///
/// # fn main() {
/// # use self::crates::dsl::*;
/// let target_name = "diesel";
/// crates.filter(canon_crate_name(name).eq(canon_crate_name(target_name)));
/// // This will generate the following SQL
/// // SELECT * FROM crates WHERE canon_crate_name(crates.name) = canon_crate_name($1)
/// # }
macro_rules! sql_function {
    ($fn_name:ident, $struct_name:ident, ($($arg_name:ident: $arg_type:ty),*) -> $return_type:ty) => {
        sql_function!($fn_name, $struct_name, ($($arg_name: $arg_type),*) -> $return_type, "");
    };

    ($fn_name:ident, $struct_name:ident, ($($arg_name:ident: $arg_type:ty),*) -> $return_type:ty,
    $docs: expr) => {
        sql_function_body!($fn_name, $struct_name, ($($arg_name: $arg_type),*) -> $return_type, $docs);
    };

    ($fn_name:ident, $struct_name:ident, ($($arg_name:ident: $arg_type:ty),*)) => {
        sql_function!($fn_name, $struct_name, ($($arg_name: $arg_type),*) -> ());
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! no_arg_sql_function_body_except_to_sql {
    ($type_name:ident, $return_type:ty, $docs:expr) => {
        #[allow(non_camel_case_types)]
        #[doc=$docs]
        pub struct $type_name;

        impl $crate::expression::Expression for $type_name {
            type SqlType = $return_type;
        }

        impl<QS> $crate::expression::SelectableExpression<QS> for $type_name {
            type SqlTypeForSelect = $return_type;
        }

        impl<QS> $crate::expression::AppearsOnTable<QS> for $type_name {
        }

        impl $crate::expression::NonAggregate for $type_name {
        }

        impl_query_id!($type_name);
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! no_arg_sql_function_body {
    ($type_name:ident, $return_type:ty, $docs:expr, $($constraint:ident)::+) => {
        no_arg_sql_function_body_except_to_sql!($type_name, $return_type, $docs);

        impl<DB> $crate::query_builder::QueryFragment<DB> for $type_name where
            DB: $crate::backend::Backend + $($constraint)::+,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                use $crate::query_builder::QueryBuilder;
                out.push_sql(concat!(stringify!($type_name), "()"));
                Ok(())
            }

            fn collect_binds(&self, _out: &mut DB::BindCollector) -> $crate::result::QueryResult<()> {
                Ok(())
            }

            fn is_safe_to_cache_prepared(&self) -> bool {
                true
            }
        }
    };

    ($type_name:ident, $return_type:ty, $docs:expr) => {
        no_arg_sql_function_body_except_to_sql!($type_name, $return_type, $docs);

        impl<DB> $crate::query_builder::QueryFragment<DB> for $type_name where
            DB: $crate::backend::Backend,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> $crate::query_builder::BuildQueryResult {
                use $crate::query_builder::QueryBuilder;
                out.push_sql(concat!(stringify!($type_name), "()"));
                Ok(())
            }

            fn collect_binds(&self, _out: &mut DB::BindCollector) -> $crate::result::QueryResult<()> {
                Ok(())
            }

            fn is_safe_to_cache_prepared(&self) -> bool {
                true
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
/// no_arg_sql_function!(now, types::Timestamp, "Represents the SQL NOW() function");
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

pub mod aggregate_ordering;
pub mod aggregate_folding;
pub mod date_and_time;
