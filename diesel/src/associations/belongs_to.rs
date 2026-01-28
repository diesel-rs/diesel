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
/// # Unrelated Rows
///
/// When using [`GroupedBy::grouped_by`], if the child rows were not queried
/// using the provided parent rows, it is not guaranteed a parent row
/// will be found for a given child row.
/// This is possible, if the foreign key in the relationship is nullable,
/// or if a child row's parent was not present in the provided slice,
/// in which case, unrelated child rows will be discarded.
///
/// If discarding these rows is undesirable, it may be preferable to use
/// [`GroupedBy::try_grouped_by`].
///
/// # Handling Duplicate Parent Rows
///
/// Both [`GroupedBy::grouped_by`] and [`GroupedBy::try_grouped_by`]
/// expect all of the elements of `parents` to produce a unique value
/// when calling [`Identifiable::id`].
/// If this is not true, child rows may be added to an unexpected index.
///
/// As a result, it is recommended to use [`QueryDsl::distinct`]
/// or [`slice::sort`] and [`Vec::dedup`],
/// to ensure the elements of `parents` are unique.
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
///         User {
///             id: 1,
///             name: "Sean".into(),
///         },
///         vec![
///             Post {
///                 id: 1,
///                 user_id: 1,
///                 title: "My first post".into(),
///             },
///             Post {
///                 id: 2,
///                 user_id: 1,
///                 title: "About Rust".into(),
///             },
///         ],
///     ),
///     (
///         User {
///             id: 2,
///             name: "Tess".into(),
///         },
///         vec![Post {
///             id: 3,
///             user_id: 2,
///             title: "My first post too".into(),
///         }],
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

    /// A fallible alternative to [`GroupedBy::grouped_by`].
    ///
    /// If any child record could not be grouped,
    /// either because of a `NULL` foreign key,
    /// or a parent record with a matching key could not be found,
    /// this function should return `Err`,
    /// with all successfully grouped records, as well as any ungrouped records.
    ///
    /// # Errors
    ///
    /// If a parent record could not be found for any of the child records,
    /// this function should return the `TryGroupedByError`.
    /// Every supplied record should be contained in the returned error,
    /// either in the `grouped` field, if it was successfully grouped,
    /// or the `ungrouped` field, if it was not possible to associate
    /// with a parent record.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// # use diesel::associations::TryGroupedByError;
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
    /// let mut posts = Post::belonging_to(&users).load::<Post>(connection)?;
    /// posts.push(Post {
    ///     id: 9,
    ///     user_id: 42,
    ///     title: "A post returned from another query".into(),
    /// });
    /// let TryGroupedByError {
    ///     grouped, ungrouped, ..
    /// } = posts.try_grouped_by(&users).unwrap_err();
    ///
    /// let grouped_data = users.into_iter().zip(grouped).collect::<Vec<_>>();
    ///
    /// let expected_grouped_data = vec![
    ///     (
    ///         User {
    ///             id: 1,
    ///             name: "Sean".into(),
    ///         },
    ///         vec![
    ///             Post {
    ///                 id: 1,
    ///                 user_id: 1,
    ///                 title: "My first post".into(),
    ///             },
    ///             Post {
    ///                 id: 2,
    ///                 user_id: 1,
    ///                 title: "About Rust".into(),
    ///             },
    ///         ],
    ///     ),
    ///     (
    ///         User {
    ///             id: 2,
    ///             name: "Tess".into(),
    ///         },
    ///         vec![Post {
    ///             id: 3,
    ///             user_id: 2,
    ///             title: "My first post too".into(),
    ///         }],
    ///     ),
    /// ];
    ///
    /// let expected_ungrouped_data = vec![Post {
    ///     id: 9,
    ///     user_id: 42,
    ///     title: "A post returned from another query".into(),
    /// }];
    ///
    /// assert_eq!(expected_grouped_data, grouped_data);
    /// assert_eq!(expected_ungrouped_data, ungrouped);
    /// #     Ok(())
    /// # }
    /// ```
    fn try_grouped_by(
        self,
        parents: &'a [Parent],
    ) -> Result<Vec<Vec<Self::Item>>, TryGroupedByError<Self::Item>> {
        Ok(self.grouped_by(parents))
    }
}

/// A type of error which can be returned when attempting to group
/// a list of records.
///
/// If a child record has a nullable foreign key, or is being grouped
/// using a different relationship to the one that was used to query it,
/// it may not be possible to find a parent record it should be grouped by.
///
/// When encountering these missing relationships,
/// it may still be possible to group remaining records,
/// but would affect the contents of the returned values.
/// By extracting the contents of this struct, it is still possible
/// to use these resulting groups, as well as any records
/// that could not be grouped.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[non_exhaustive]
pub struct TryGroupedByError<Child> {
    /// The collection of records which were successfully indexed
    /// against a parent record.
    pub grouped: Vec<Vec<Child>>,
    /// The collection of records that could not be indexed
    /// against a parent record.
    pub ungrouped: Vec<Child>,
}

type Id<T> = <T as Identifiable>::Id;

impl<Child> TryGroupedByError<Child> {
    /// Creates a `TryGroupedByError`.
    ///
    /// This is generally used by methods like [`GroupedBy::try_grouped_by`].
    pub fn new(grouped: Vec<Vec<Child>>, ungrouped: Vec<Child>) -> Self {
        Self { grouped, ungrouped }
    }
}

impl<'a, Parent: 'a, Child, Iter> GroupedBy<'a, Parent> for Iter
where
    Iter: IntoIterator<Item = Child>,
    Child: BelongsTo<Parent>,
    &'a Parent: Identifiable,
    Id<&'a Parent>: Borrow<Child::ForeignKey>,
{
    fn grouped_by(self, parents: &'a [Parent]) -> Vec<Vec<Child>> {
        use std::collections::HashMap;
        use std::iter;

        let mut grouped: Vec<_> = iter::repeat_with(Vec::new).take(parents.len()).collect();

        let id_indices: HashMap<_, _> = parents
            .iter()
            .enumerate()
            .map(|(i, u)| (u.id(), i))
            .collect();

        self.into_iter()
            .filter_map(|child| {
                let fk = child.foreign_key()?;
                let i = id_indices.get(fk)?;

                Some((i, child))
            })
            .for_each(|(i, child)| grouped[*i].push(child));

        grouped
    }

    fn try_grouped_by(
        self,
        parents: &'a [Parent],
    ) -> Result<Vec<Vec<Child>>, TryGroupedByError<Child>> {
        use std::collections::HashMap;
        use std::iter;

        let mut grouped: Vec<_> = iter::repeat_with(Vec::new).take(parents.len()).collect();
        let mut ungrouped: Vec<_> = Vec::new();

        let id_indices: HashMap<_, _> = parents
            .iter()
            .enumerate()
            .map(|(i, u)| (u.id(), i))
            .collect();

        for child in self {
            child
                .foreign_key()
                .and_then(|i| id_indices.get(i))
                .map_or(&mut ungrouped, |i| &mut grouped[*i])
                .push(child);
        }

        if ungrouped.is_empty() {
            Ok(grouped)
        } else {
            Err(TryGroupedByError::new(grouped, ungrouped))
        }
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
