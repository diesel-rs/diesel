use crate::associations::{HasTable, Identifiable};
use crate::dsl::Find;
use crate::query_dsl::methods::FindDsl;
use crate::query_source::Table;

#[doc(hidden)]
#[derive(Debug)]
pub struct UpdateTarget<Table, WhereClause> {
    pub table: Table,
    pub where_clause: WhereClause,
}

/// A type which can be passed to [`update`] or [`delete`].
///
/// Apps will never need to implement this type directly. There are three kinds
/// which implement this trait. Tables, queries which have only had `filter`
/// called on them, and types which implement `Identifiable`.
///
/// When a table is passed to `update`, every row in the table will be updated.
/// You can scope this down by calling [`filter`] which will
/// result in `UPDATE your_table SET ... WHERE args_to_filter`. Passing a type
/// which implements `Identifiable` is the same as passing
/// `SomeStruct::table().find(some_struct)`.
///
/// [`update`]: crate::update()
/// [`delete`]: crate::delete()
/// [`filter`]: crate::query_builder::UpdateStatement::filter()
pub trait IntoUpdateTarget: HasTable {
    /// What is the `WHERE` clause of this target?
    type WhereClause;

    /// Decomposes `self` into the table and where clause.
    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause>;
}

impl<T, Tab, V> IntoUpdateTarget for T
where
    T: Identifiable<Table = Tab>,
    Tab: Table + FindDsl<T::Id>,
    Find<Tab, T::Id>: IntoUpdateTarget<Table = Tab, WhereClause = V>,
{
    type WhereClause = V;

    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause> {
        T::table().find(self.id()).into_update_target()
    }
}
