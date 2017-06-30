use associations::{Identifiable, HasTable};
use helper_types::Find;
use query_dsl::FindDsl;

#[doc(hidden)]
#[derive(Debug)]
pub struct UpdateTarget<Table, WhereClause> {
    pub table: Table,
    pub where_clause: WhereClause,
}

/// A type which can be passed to `update`.
///
/// Apps will never need to implement this type directly. There are three kinds
/// of things which implement this trait. Tables, queries which have only had
/// `filter` called on them, and types which implement `Identifiable`.
///
/// When a table is passed to `update`, every row in the table will be updated.
/// You can scope this down by calling `filter` before passing it, which will
/// result in `UPDATE your_table SET ... WHERE args_to_filter`. Passing a type
/// which implements `Identifiable` is the same as passing
/// `SomeStruct::table().find(some_struct)`.
pub trait IntoUpdateTarget: HasTable {
    type WhereClause;

    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause>;
}

impl<T: Identifiable, V> IntoUpdateTarget for T where
    T::Table: FindDsl<T::Id>,
    Find<T::Table, T::Id>: IntoUpdateTarget<Table=T::Table, WhereClause=V>,
{
    type WhereClause = V;

    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause> {
        T::table().find(self.id()).into_update_target()
    }
}
