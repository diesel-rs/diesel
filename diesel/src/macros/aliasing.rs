#[macro_export]
macro_rules! diesel_define_alias {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, QueryId)]
        #[allow(non_camel_case_types)]
        pub struct $name;

        impl $crate::query_source::AppearsInFromClause<$name> for $name {
            type Count = $crate::query_source::Once;
        }

        impl $crate::query_source::AppearsInFromClause<$name> for () {
            type Count = $crate::query_source::Never;
        }

        impl<DB> $crate::query_builder::QueryFragment<DB> for $name
        where
            DB: $crate::backend::Backend,
            $crate::query_builder::nodes::Identifier<'static>: $crate::query_builder::QueryFragment<DB>,
        {
            fn walk_ast(&self, out: $crate::query_builder::AstPass<DB>) -> $crate::result::QueryResult<()> {
                $crate::query_builder::QueryFragment::walk_ast(
                    &$crate::query_builder::nodes::Identifier(stringify!($name)),
                    out,
                )
            }
        }
    };
}
