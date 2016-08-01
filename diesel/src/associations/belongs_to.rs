use expression::AsExpression;
use expression::helper_types::{Eq, EqAny};
use expression::array_comparison::AsInExpression;
use helper_types::{FindBy, Filter};
use prelude::*;
use super::Identifiable;

pub trait BelongsTo<Parent: Identifiable> {
    type ForeignKeyColumn: Column;

    fn foreign_key(&self) -> Parent::Id;
    fn foreign_key_column() -> Self::ForeignKeyColumn;
}

pub trait GroupedBy<Parent>: IntoIterator + Sized {
    fn grouped_by(self, parents: &[Parent]) -> Vec<Vec<Self::Item>>;
}

impl<Parent, Child, Iter> GroupedBy<Parent> for Iter where
    Iter: IntoIterator<Item=Child>,
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

impl<Parent, Child> BelongingToDsl<Parent> for Child where
    Parent: Identifiable,
    Child: Identifiable + BelongsTo<Parent>,
    Parent::Id: AsExpression<<Child::ForeignKeyColumn as Expression>::SqlType>,
    <Child as Identifiable>::Table: FilterDsl<Eq<Child::ForeignKeyColumn, Parent::Id>>,
{
    type Output = FindBy<
        Child::Table,
        Child::ForeignKeyColumn,
        Parent::Id,
    >;

    fn belonging_to(parent: &Parent) -> Self::Output {
        Child::table().filter(Child::foreign_key_column().eq(parent.id()))
    }
}

impl<Parent, Child> BelongingToDsl<[Parent]> for Child where
    Parent: Identifiable,
    Child: Identifiable + BelongsTo<Parent>,
    Vec<Parent::Id>: AsInExpression<<Child::ForeignKeyColumn as Expression>::SqlType>,
    <Child as Identifiable>::Table: FilterDsl<EqAny<Child::ForeignKeyColumn, Vec<Parent::Id>>>,
{
    type Output = Filter<
        Child::Table,
        EqAny<
            Child::ForeignKeyColumn,
            Vec<Parent::Id>,
        >,
    >;

    fn belonging_to(parents: &[Parent]) -> Self::Output {
        let ids = parents.iter().map(Parent::id).collect::<Vec<_>>();
        Child::table().filter(Child::foreign_key_column().eq_any(ids))
    }
}

impl<Parent, Child> BelongingToDsl<Vec<Parent>> for Child where
    Child: BelongingToDsl<[Parent]>,
{
    type Output = Child::Output;

    fn belonging_to(parents: &Vec<Parent>) -> Self::Output {
        Self::belonging_to(&**parents)
    }
}
