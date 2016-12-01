use associations::{Identifiable, HasTable};
use helper_types::Find;
use query_dsl::FindDsl;

#[doc(hidden)]
#[derive(Debug)]
pub struct UpdateTarget<Table, WhereClause> {
    pub table: Table,
    pub where_clause: Option<WhereClause>,
}

pub trait IntoUpdateTarget: HasTable {
    type WhereClause;

    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause>;
}

impl<'a, T: Identifiable, V> IntoUpdateTarget for &'a T where
    T::Table: FindDsl<&'a T::Id>,
    Find<T::Table, &'a T::Id>: IntoUpdateTarget<Table=T::Table, WhereClause=V>,
{
    type WhereClause = V;

    fn into_update_target(self) -> UpdateTarget<Self::Table, Self::WhereClause> {
        T::table().find(self.id()).into_update_target()
    }
}
