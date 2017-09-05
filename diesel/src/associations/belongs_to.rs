use expression::AsExpression;
use expression::helper_types::{Eq, EqAny};
use expression::array_comparison::AsInExpression;
use helper_types::{Filter, FindBy};
use prelude::*;
use super::{HasTable, Identifiable};

use std::borrow::Borrow;
use std::hash::Hash;

pub trait BelongsTo<Parent> {
    type ForeignKey: Hash + ::std::cmp::Eq;
    type ForeignKeyColumn: Column;

    fn foreign_key(&self) -> Option<&Self::ForeignKey>;
    fn foreign_key_column() -> Self::ForeignKeyColumn;
}

pub trait GroupedBy<'a, Parent>: IntoIterator + Sized {
    fn grouped_by(self, parents: &'a [Parent]) -> Vec<Vec<Self::Item>>;
}

type Id<T> = <T as Identifiable>::Id;

impl<'a, Parent: 'a, Child, Iter> GroupedBy<'a, Parent> for Iter
where
    Iter: IntoIterator<Item = Child>,
    Child: BelongsTo<Parent>,
    &'a Parent: Identifiable,
    Id<&'a Parent>: Borrow<Child::ForeignKey>,
{
    fn grouped_by(self, parents: &'a [Parent]) -> Vec<Vec<Child>> {
        use std::collections::HashMap;

        let id_indices: HashMap<_, _> = parents
            .iter()
            .enumerate()
            .map(|(i, u)| (u.id(), i))
            .collect();
        let mut result = parents.iter().map(|_| Vec::new()).collect::<Vec<_>>();
        for child in self {
            if let Some(index) = child.foreign_key().map(|i| id_indices[i]) {
                result[index].push(child);
            }
        }
        result
    }
}

impl<'a, Parent, Child> BelongingToDsl<&'a Parent> for Child
where
    &'a Parent: Identifiable,
    Child: HasTable + BelongsTo<Parent>,
    Id<&'a Parent>: AsExpression<<Child::ForeignKeyColumn as Expression>::SqlType>,
    <Child as HasTable>::Table: FilterDsl<Eq<Child::ForeignKeyColumn, Id<&'a Parent>>>,
    Child::ForeignKeyColumn: ExpressionMethods,
{
    type Output = FindBy<Child::Table, Child::ForeignKeyColumn, Id<&'a Parent>>;

    fn belonging_to(parent: &'a Parent) -> Self::Output {
        Child::table().filter(Child::foreign_key_column().eq(parent.id()))
    }
}

impl<'a, Parent, Child> BelongingToDsl<&'a [Parent]> for Child
where
    &'a Parent: Identifiable,
    Child: HasTable + BelongsTo<Parent>,
    Vec<Id<&'a Parent>>: AsInExpression<<Child::ForeignKeyColumn as Expression>::SqlType>,
    <Child as HasTable>::Table: FilterDsl<EqAny<Child::ForeignKeyColumn, Vec<Id<&'a Parent>>>>,
    Child::ForeignKeyColumn: ExpressionMethods,
{
    type Output = Filter<Child::Table, EqAny<Child::ForeignKeyColumn, Vec<Id<&'a Parent>>>>;

    fn belonging_to(parents: &'a [Parent]) -> Self::Output {
        let ids = parents.iter().map(Identifiable::id).collect::<Vec<_>>();
        Child::table().filter(Child::foreign_key_column().eq_any(ids))
    }
}

impl<'a, Parent, Child> BelongingToDsl<&'a Vec<Parent>> for Child
where
    Child: BelongingToDsl<&'a [Parent]>,
{
    type Output = Child::Output;

    fn belonging_to(parents: &'a Vec<Parent>) -> Self::Output {
        Self::belonging_to(&**parents)
    }
}
