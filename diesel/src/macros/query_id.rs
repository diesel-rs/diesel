#[macro_export]
#[doc(hidden)]
macro_rules! impl_query_id {
    ($name: ident) => {
        impl $crate::query_builder::QueryId for $name {
            type QueryId = Self;

            fn has_static_query_id() -> bool {
                true
            }
        }
    };

    ($name: ident<$($ty_param: ident),+>) => {
        #[allow(non_camel_case_types)]
        impl<$($ty_param),*> $crate::query_builder::QueryId for $name<$($ty_param),*> where
            $($ty_param: $crate::query_builder::QueryId),*
        {
            type QueryId = $name<$($ty_param::QueryId),*>;

            fn has_static_query_id() -> bool {
                $($ty_param::has_static_query_id() &&)* true
            }
        }
    };

    (noop: $name: ident) => {
        impl $crate::query_builder::QueryId for $name {
            type QueryId = ();

            fn has_static_query_id() -> bool {
                false
            }
        }
    };

    (noop: $name: ident<$($ty_param: ident),+>) => {
        #[allow(non_camel_case_types)]
        impl<$($ty_param),*> $crate::query_builder::QueryId for $name<$($ty_param),*> {
            type QueryId = ();

            fn has_static_query_id() -> bool {
                false
            }
        }
    }
}
