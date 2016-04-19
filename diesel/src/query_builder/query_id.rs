use std::any::{Any, TypeId};
use super::QueryFragment;

pub trait QueryId {
    type QueryId: Any;
    fn has_static_query_id() -> bool;

    fn query_id() -> Option<TypeId> {
        if Self::has_static_query_id() {
            Some(TypeId::of::<Self::QueryId>())
        } else {
            None
        }
    }
}

impl QueryId for () {
    type QueryId = ();

    fn has_static_query_id() -> bool {
        true
    }
}

impl<T: QueryId + ?Sized> QueryId for Box<T> {
    type QueryId = T::QueryId;

    fn has_static_query_id() -> bool {
        T::has_static_query_id()
    }
}


impl<'a, T: QueryId + ?Sized> QueryId for &'a T {
    type QueryId = T::QueryId;

    fn has_static_query_id() -> bool {
        T::has_static_query_id()
    }
}

impl<DB> QueryId for QueryFragment<DB> {
    type QueryId = ();

    fn has_static_query_id() -> bool {
        false
    }
}

#[macro_export]
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

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use backend::Debug;
    use prelude::*;
    use super::QueryId;

    table! {
        users {
            id -> Integer,
            name -> VarChar,
        }
    }

    fn query_id<T: QueryId>(_: T) -> Option<TypeId> {
        T::query_id()
    }

    #[test]
    fn queries_with_no_dynamic_elements_have_a_static_id() {
        use self::users::dsl::*;
        assert!(query_id(users).is_some());
        assert!(query_id(users.select(name)).is_some());
        assert!(query_id(users.filter(name.eq("Sean"))).is_some());
    }

    #[test]
    fn queries_with_different_types_have_different_ids() {
        let id1 = query_id(users::table.select(users::name));
        let id2 = query_id(users::table.select(users::id));
        assert!(id1 != id2);
    }

    #[test]
    fn bind_params_use_only_sql_type_for_query_id() {
        use self::users::dsl::*;
        let id1 = query_id(users.filter(name.eq("Sean")));
        let id2 = query_id(users.filter(name.eq("Tess".to_string())));

        assert_eq!(id1, id2);
    }

    #[test]
    fn boxed_queries_do_not_have_static_query_id() {
        assert!(query_id(users::table.into_boxed::<Debug>()).is_none());
    }
}
