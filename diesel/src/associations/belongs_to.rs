use super::HasTable;
use crate::dsl::{Eq, EqAny, Filter, FindBy};
use crate::expression::array_comparison::AsInExpression;
use crate::expression::AsExpression;
use crate::prelude::*;
use crate::query_dsl::methods::FilterDsl;
use crate::sql_types::SqlType;

use std::borrow::Borrow;
use std::hash::Hash;

/// Indicates that a type belongs to `Parent`
///
/// Specifically, this means that this struct has fields
/// which correspond to the primary key of `Parent`.
/// This implies that a foreign key relationship exists on the tables.
///
/// This trait is not capable of supporting composite foreign keys
pub trait BelongsTo<Parent> {
    /// The foreign key of this struct
    type ForeignKey: Hash + ::std::cmp::Eq;
    /// The database column representing the foreign key
    /// of the table this struct represents
    type ForeignKeyColumn: Column;

    /// Returns the foreign key for `self`
    fn foreign_key(&self) -> Option<&Self::ForeignKey>;
    /// Returns the foreign key column of this struct's table
    fn foreign_key_column() -> Self::ForeignKeyColumn;
}

/// The `grouped_by` function groups records by their parent.
///
/// `grouped_by` is called on a `Vec<Child>` with a `&[Parent]`.
/// The return value will be `Vec<Vec<Child>>` indexed to match their parent.
/// Or to put it another way, the returned data can be passed to `zip`,
/// and it will be combined with its parent.
/// This function does not generate a `GROUP BY` SQL statement,
/// as it operates on data structures already loaded from the database
///
/// **Child** refers to the "many" part of a "one to many" relationship. It "belongs to" its parent
/// **Parent** refers to the "one" part of a "one to many" relationship and can "have many" children.
/// The child always has a foreign key, which refers to its parent's primary key.
/// In the following relationship, User has many Posts,
/// so User is the parent and Posts are children.
///
/// # Example
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// # use schema::{posts, users};
/// #
/// # #[derive(Identifiable, Queryable, PartialEq, Debug)]
/// # pub struct User {
/// #     id: i32,
/// #     name: String,
/// # }
/// #
/// # #[derive(Debug, PartialEq)]
/// # #[derive(Identifiable, Queryable, Associations)]
/// # #[diesel(belongs_to(User))]
/// # pub struct Post {
/// #     id: i32,
/// #     user_id: i32,
/// #     title: String,
/// # }
/// #
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// let users = users::table.load::<User>(connection)?;
/// let posts = Post::belonging_to(&users)
///     .load::<Post>(connection)?
///     .grouped_by(&users);
/// let data = users.into_iter().zip(posts).collect::<Vec<_>>();
///
/// let expected_data = vec![
///     (
///         User { id: 1, name: "Sean".into() },
///         vec![
///             Post { id: 1, user_id: 1, title: "My first post".into() },
///             Post { id: 2, user_id: 1, title: "About Rust".into() },
///         ],
///     ),
///     (
///         User { id: 2, name: "Tess".into() },
///         vec![
///             Post { id: 3, user_id: 2, title: "My first post too".into() },
///         ],
///     ),
/// ];
///
/// assert_eq!(expected_data, data);
/// #     Ok(())
/// # }
/// ```
///
/// See [the module documentation] for more examples
///
/// [the module documentation]: super
pub trait GroupedBy<'a, Parent>: IntoIterator + Sized {
    /// See the trait documentation.
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
    Child::Table: FilterDsl<Eq<Child::ForeignKeyColumn, Id<&'a Parent>>>,
    Child::ForeignKeyColumn: ExpressionMethods,
    <Child::ForeignKeyColumn as Expression>::SqlType: SqlType,
{
    type Output = FindBy<Child::Table, Child::ForeignKeyColumn, Id<&'a Parent>>;

    fn belonging_to(parent: &'a Parent) -> Self::Output {
        FilterDsl::filter(Child::table(), Child::foreign_key_column().eq(parent.id()))
    }
}

impl<'a, Parent, Child> BelongingToDsl<&'a [Parent]> for Child
where
    &'a Parent: Identifiable,
    Child: HasTable + BelongsTo<Parent>,
    Vec<Id<&'a Parent>>: AsInExpression<<Child::ForeignKeyColumn as Expression>::SqlType>,
    <Child as HasTable>::Table: FilterDsl<EqAny<Child::ForeignKeyColumn, Vec<Id<&'a Parent>>>>,
    Child::ForeignKeyColumn: ExpressionMethods,
    <Child::ForeignKeyColumn as Expression>::SqlType: SqlType,
{
    type Output = Filter<Child::Table, EqAny<Child::ForeignKeyColumn, Vec<Id<&'a Parent>>>>;

    fn belonging_to(parents: &'a [Parent]) -> Self::Output {
        let ids = parents.iter().map(Identifiable::id).collect::<Vec<_>>();
        FilterDsl::filter(Child::table(), Child::foreign_key_column().eq_any(ids))
    }
}

impl<'a, Parent, Child> BelongingToDsl<(&'a [Parent], &'a [Parent])> for Child
where
    &'a Parent: Identifiable,
    Child: HasTable + BelongsTo<Parent>,
    Vec<Id<&'a Parent>>: AsInExpression<<Child::ForeignKeyColumn as Expression>::SqlType>,
    <Child as HasTable>::Table: FilterDsl<EqAny<Child::ForeignKeyColumn, Vec<Id<&'a Parent>>>>,
    Child::ForeignKeyColumn: ExpressionMethods,
    <Child::ForeignKeyColumn as Expression>::SqlType: SqlType,
{
    type Output = Filter<Child::Table, EqAny<Child::ForeignKeyColumn, Vec<Id<&'a Parent>>>>;

    fn belonging_to(parents: (&'a [Parent], &'a [Parent])) -> Self::Output {
        let ids = parents
            .0
            .iter()
            .chain(parents.1.iter())
            .map(Identifiable::id)
            .collect::<Vec<_>>();
        FilterDsl::filter(Child::table(), Child::foreign_key_column().eq_any(ids))
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
