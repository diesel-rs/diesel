use std::hash::Hash;
use query_source::Column;

pub trait Identifiable {
    type Id: Hash + Eq + Copy;

    fn id(&self) -> Self::Id;
}

pub trait BelongsTo<Parent: Identifiable, FK> where FK: Column
{
    fn foreign_key(&self) -> Parent::Id;
}

pub trait GroupedBy<Parent, FK>: IntoIterator + Sized {
    fn grouped_by(self, parents: &[Parent]) -> Vec<Vec<Self::Item>>;
}

impl<Parent, Child, FK> GroupedBy<Parent, FK> for Vec<Child> where
    FK: Column,
    Child: BelongsTo<Parent, FK>,
    Parent: Identifiable
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
