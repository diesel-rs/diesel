use dsl::{Eq, EqAny, Filter, FindBy};
use expression::AsExpression;
use expression::array_comparison::AsInExpression;
use query_dsl::methods::FilterDsl;
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

/// The `grouped_by` function groups records by their parent.
///
/// `grouped_by` is called on a `Vec<Child>` with a `&Vec<Parent>` and returns a `Vec<Vec<Child>>`
/// where the index of the children matches the index of the parent they belong to. This function
/// does not generate a `GROUP BY` SQL statement, as it operates on data structures already loaded
/// from the database backend.
///
/// **Child** refers to the *many* part of a *one to many* relationship and has *one parent*.
/// **Parent** refers to the *one* part of a *one to many* relationship and can *have many children*.
/// In the following relationship, User has many Posts,
/// so User is the parent and Posts are children.
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # include!("../doctest_setup.rs");
/// # use schema::users;
/// # use schema::posts;
/// #
/// # #[derive(Debug, Identifiable, Queryable)]
/// # pub struct User {
/// #     id: i32,
/// #     name: String,
/// # }
/// #
/// # #[derive(Debug, PartialEq, Identifiable, Queryable, Associations)]
/// # #[belongs_to(User)]
/// # pub struct Post {
/// #     id: i32,
/// #     user_id: i32,
/// #     title: String,
/// # }
/// #
/// # fn main() {
/// #   use users::dsl::*;
/// #   use posts::dsl::*;
/// #   let connection = establish_connection();
/// #
/// let user_list = users.load::<User>(&connection).expect("Couldn't load users");
/// let post_list = posts.load::<Post>(&connection).expect("Couldn't load posts");
///
/// // Group Posts by Users
/// let posts_grouped_by_user: Vec<Vec<Post>> = post_list.grouped_by(&user_list);
/// let expected = vec![
///     vec![
///         Post { id: 1, user_id: 1, title: "My first post".to_string() },
///         Post { id: 2, user_id: 1, title: "About Rust".to_string() }
///     ],
///     vec![
///         Post { id: 3, user_id: 2, title: "My first post too".to_string() }
///     ]
/// ];
///
/// assert_eq!(posts_grouped_by_user, expected);
/// # }
/// ```
///
/// View the [associations] doc for more `grouped_by()` code examples
///
/// [associations]: ../associations/index.html
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
    Child::Table: FilterDsl<Eq<Child::ForeignKeyColumn, Id<&'a Parent>>>,
    Child::ForeignKeyColumn: ExpressionMethods,
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
{
    type Output = Filter<Child::Table, EqAny<Child::ForeignKeyColumn, Vec<Id<&'a Parent>>>>;

    fn belonging_to(parents: &'a [Parent]) -> Self::Output {
        let ids = parents.iter().map(Identifiable::id).collect::<Vec<_>>();
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
