#[macro_export]
#[doc(hidden)]
macro_rules! impl_query_id {
    ($name: ident) => {
        impl $crate::query_builder::QueryId for $name {
            type QueryId = Self;

            const HAS_STATIC_QUERY_ID: bool = true;
        }
    };

    ($name: ident<$($ty_param: ident),+>) => {
        #[allow(non_camel_case_types)]
        impl<$($ty_param),*> $crate::query_builder::QueryId for $name<$($ty_param),*> where
            $($ty_param: $crate::query_builder::QueryId),*
        {
            type QueryId = $name<$($ty_param::QueryId),*>;

            const HAS_STATIC_QUERY_ID: bool = $($ty_param::HAS_STATIC_QUERY_ID &&)* true;
        }
    };

    (noop: $name: ident) => {
        impl $crate::query_builder::QueryId for $name {
            type QueryId = ();

            const HAS_STATIC_QUERY_ID: bool = false;
        }
    };

    (noop: $name: ident<$($ty_param: ident),+>) => {
        #[allow(non_camel_case_types)]
        impl<$($ty_param),*> $crate::query_builder::QueryId for $name<$($ty_param),*> {
            type QueryId = ();

            const HAS_STATIC_QUERY_ID: bool = false;
        }
    }
}
