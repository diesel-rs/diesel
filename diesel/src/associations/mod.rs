use std::hash::Hash;

use query_dsl::FindDsl;
use query_source::Table;

pub trait Identifiable {
    type Id: Hash + Eq + Copy;
    type Table: Table + FindDsl<Self::Id>;

    fn table() -> Self::Table;
    fn id(&self) -> Self::Id;
}

pub trait BelongsTo<Parent: Identifiable> {
    fn foreign_key(&self) -> Parent::Id;
}

pub trait GroupedBy<Parent>: IntoIterator + Sized {
    fn grouped_by(self, parents: &[Parent]) -> Vec<Vec<Self::Item>>;
}

impl<Parent, Child> GroupedBy<Parent> for Vec<Child> where
    Child: BelongsTo<Parent>,
    Parent: Identifiable,
{
    fn grouped_by(self, parents: &[Parent]) -> Vec<Vec<Child>> {
        use std::collections::HashMap;

        let id_indices: HashMap<_, _> = parents.iter().enumerate().map(|(i, u)| (u.id(), i)).collect();
        let mut result = parents.iter().map(|_| Vec::new()).collect::<Vec<_>>();
        for child in self {
            let index = id_indices[&child.foreign_key()];
            result[index].push(child);
        }
        result
    }
}
