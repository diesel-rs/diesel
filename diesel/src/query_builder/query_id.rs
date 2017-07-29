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

#[cfg(test)]
mod tests {
    use std::any::TypeId;

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
        assert_ne!(id1, id2);
    }

    #[test]
    fn bind_params_use_only_sql_type_for_query_id() {
        use self::users::dsl::*;
        let id1 = query_id(users.filter(name.eq("Sean")));
        let id2 = query_id(users.filter(name.eq("Tess".to_string())));

        assert_eq!(id1, id2);
    }

    #[test]
    #[cfg(features="postgres")]
    fn boxed_queries_do_not_have_static_query_id() {
        use pg::Pg;
        assert!(query_id(users::table.into_boxed::<Pg>()).is_none());
    }
}
