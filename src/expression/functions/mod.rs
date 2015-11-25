#[macro_export]
macro_rules! sql_function {
    ($fn_name:ident, $struct_name:ident, ($($arg_name:ident: $arg_type:ident),*) -> $return_type:ident) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy)]
        pub struct $struct_name<$($arg_name),*> {
            $($arg_name: $arg_name),*
        }

        #[allow(non_camel_case_types)]
        pub type $fn_name<$($arg_name),*> = $struct_name<$(
            <$arg_name as $crate::expression::AsExpression<$crate::types::$arg_type>>::Expression
        ),*>;

        #[allow(non_camel_case_types)]
        pub fn $fn_name<$($arg_name),*>($($arg_name: $arg_name),*)
            -> $fn_name<$($arg_name),*>
            where $($arg_name: $crate::expression::AsExpression<$crate::types::$arg_type>),+
        {
            $struct_name {
                $($arg_name: $arg_name.as_expression()),+
            }
        }

        #[allow(non_camel_case_types)]
        impl<$($arg_name),*> $crate::expression::Expression for $struct_name<$($arg_name),*> where
            $($arg_name: $crate::expression::Expression),*
        {
            type SqlType = $crate::types::$return_type;

            fn to_sql(&self, out: &mut $crate::query_builder::QueryBuilder)
                -> $crate::query_builder::BuildQueryResult {
                    out.push_sql(concat!(stringify!($fn_name), "("));
                    $(try!(self.$arg_name.to_sql(out));)*
                    out.push_sql(")");
                    Ok(())
                }
        }

        #[allow(non_camel_case_types)]
        impl<$($arg_name),*, QS> $crate::expression::SelectableExpression<QS> for $struct_name<$($arg_name),*> where
            $($arg_name: $crate::expression::SelectableExpression<QS>,)*
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
macro_rules! no_arg_sql_function {
    ($type_name:ident, $return_type:ident) => {
        #[allow(non_camel_case_types)]
        pub struct $type_name;

        impl $crate::expression::Expression for $type_name {
            type SqlType = $crate::types::$return_type;

            fn to_sql(&self, out: &mut $crate::query_builder::QueryBuilder)
                -> $crate::query_builder::BuildQueryResult {
                    out.push_sql(concat!(stringify!($type_name), "()"));
                    Ok(())
                }
        }

        impl<QS> $crate::expression::SelectableExpression<QS> for $type_name {
        }

        impl $crate::expression::NonAggregate for $type_name {
        }
    }
}

pub mod date_and_time;
