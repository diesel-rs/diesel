/// This will implement `SelectableExpression` and `AppearsOnTable` for "simple"
/// composite nodes where the where clause is roughly `AllTyParams:
/// SelectableExpression<QS>, Self: Expression`.
///
/// This macro is exported because we want to be able to call it from other
/// macros that are exported, but it is not part of our public API.
#[macro_export]
#[doc(hidden)]
macro_rules! impl_selectable_expression {
    ($struct_name:ident) => {
        $crate::impl_selectable_expression!(ty_params = (), struct_ty = $struct_name,);
    };

    ($struct_name:ident<$($ty_params:ident),+>) => {
        $crate::impl_selectable_expression!(
            ty_params = ($($ty_params),+),
            struct_ty = $struct_name<$($ty_params),+>,
        );
    };

    (ty_params = ($($ty_params:ident),*), struct_ty = $struct_ty:ty,) => {
        impl<$($ty_params,)* QS> $crate::expression::SelectableExpression<QS>
            for $struct_ty where
                $struct_ty: $crate::expression::AppearsOnTable<QS>,
                $($ty_params: $crate::expression::SelectableExpression<QS>,)*
        {
        }

        impl<$($ty_params,)* QS> $crate::expression::AppearsOnTable<QS>
            for $struct_ty where
                $struct_ty: $crate::expression::Expression,
                $($ty_params: $crate::expression::AppearsOnTable<QS>,)*
        {
        }
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(feature = "custom-count-column-tables")]
macro_rules! for_each_tuple {
    ($callback:ident) => {
        diesel_derives::__diesel_for_each_tuple!($callback, env!("DIESEL_MAX_COLUMN_COUNT"));
    };
    ($callback:ident, $size: expr) => {
        diesel_derives::__diesel_for_each_tuple!($callback, $size);
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    feature = "128-column-tables"
))]
macro_rules! for_each_tuple {
    ($callback:ident) => {
        diesel_derives::__diesel_for_each_tuple!($callback, 128);
    };
    ($callback:ident, $size: expr) => {
        diesel_derives::__diesel_for_each_tuple!($callback, $size);
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    not(feature = "128-column-tables"),
    feature = "64-column-tables"
))]
macro_rules! for_each_tuple {
    ($callback:ident) => {
        diesel_derives::__diesel_for_each_tuple!($callback, 64);
    };
    ($callback:ident, $size: expr) => {
        diesel_derives::__diesel_for_each_tuple!($callback, $size);
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    not(feature = "128-column-tables"),
    not(feature = "64-column-tables"),
    feature = "32-column-tables"
))]
macro_rules! for_each_tuple {
    ($callback:ident) => {
        diesel_derives::__diesel_for_each_tuple!($callback, 32);
    };
    ($callback:ident, $size: expr) => {
        diesel_derives::__diesel_for_each_tuple!($callback, $size);
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(all(
    not(feature = "custom-count-column-tables"),
    not(feature = "128-column-tables"),
    not(feature = "64-column-tables"),
    not(feature = "32-column-tables")
))]
macro_rules! for_each_tuple {
    ($callback:ident) => {
        diesel_derives::__diesel_for_each_tuple!($callback, 16);
    };
    ($callback:ident, $size: expr) => {
        diesel_derives::__diesel_for_each_tuple!($callback, $size);
    };
}
