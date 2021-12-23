//! Traits related to relationships between multiple tables.
//!
//! Associations in Diesel are always child-to-parent.
//! You can declare an association between two records with `#[diesel(belongs_to)]`.
//! Unlike other ORMs, Diesel has no concept of `has many`
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use schema::{posts, users};
//!
//! #[derive(Identifiable, Queryable, PartialEq, Debug)]
//! #[diesel(table_name = users)]
//! pub struct User {
//!     id: i32,
//!     name: String,
//! }
//!
//! #[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
//! #[diesel(belongs_to(User))]
//! #[diesel(table_name = posts)]
//! pub struct Post {
//!     id: i32,
//!     user_id: i32,
//!     title: String,
//! }
//!
//! # fn main() {
//! #     run_test().unwrap();
//! # }
//! #
//! # fn run_test() -> QueryResult<()> {
//! #     let connection = &mut establish_connection();
//! #     use self::users::dsl::*;
//! let user = users.find(2).get_result::<User>(connection)?;
//! let users_post = Post::belonging_to(&user)
//!     .first(connection)?;
//! let expected = Post { id: 3, user_id: 2, title: "My first post too".into() };
//! assert_eq!(expected, users_post);
//! #     Ok(())
//! # }
//! ```
//!
//! Note that in addition to the `#[diesel(belongs_to)]` annotation, we also need to
//! `#[derive(Associations)]`
//!
//! `#[diesel(belongs_to)]` is given the name of the struct that represents the parent.
//! Both the parent and child must implement [`Identifiable`].
//! The struct given to `#[diesel(belongs_to)]` must be in scope,
//! so you will need `use some_module::User` if `User` is defined in another module.
//!
//! If the parent record is generic over lifetimes, they can be written as `'_`.
//! You will also need to wrap the type in quotes until
//! `unrestricted_attribute_tokens` is stable.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! # use schema::{posts, users};
//! # use std::borrow::Cow;
//! #
//! #[derive(Identifiable)]
//! #[diesel(table_name = users)]
//! pub struct User<'a> {
//!     id: i32,
//!     name: Cow<'a, str>,
//! }
//!
//! #[derive(Associations)]
//! #[diesel(belongs_to(User<'_>))]
//! #[diesel(table_name = posts)]
//! pub struct Post {
//!     id: i32,
//!     user_id: i32,
//!     title: String,
//! }
//! #
//! # fn main() {}
//! ```
//!
//!
//! By default, Diesel assumes that your foreign keys will follow the convention `table_name_id`.
//! If your foreign key has a different name,
//! you can provide the `foreign_key` argument to `#[diesel(belongs_to)]`.
//! For example, `#[diesel(belongs_to(Foo, foreign_key = mykey))]`.
//!
//! Associated data is typically loaded in multiple queries (one query per table).
//! This is usually more efficient than using a join,
//! especially if 3 or more tables are involved.
//! For most datasets,
//! using a join to load in a single query transmits so much duplicate data
//! that it costs more time than the extra round trip would have.
//!
//! You can load the children for one or more parents using
//! [`belonging_to`]
//!
//! [`belonging_to`]: crate::query_dsl::BelongingToDsl::belonging_to
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! # use schema::users;
//! # use schema::posts;
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable)]
//! # pub struct User {
//! #     id: i32,
//! #     name: String,
//! # }
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable, Associations)]
//! # #[diesel(belongs_to(User))]
//! # pub struct Post {
//! #     id: i32,
//! #     user_id: i32,
//! #     title: String,
//! # }
//! #
//! # fn main() {
//! #   use self::users::dsl::*;
//! #   let connection = &mut establish_connection();
//! #
//! let user = users.find(1).first::<User>(connection).expect("Error loading user");
//! let post_list = Post::belonging_to(&user)
//!     .load::<Post>(connection)
//!     .expect("Error loading posts");
//! let expected = vec![
//!     Post { id: 1, user_id: 1, title: "My first post".to_string() },
//!     Post { id: 2, user_id: 1, title: "About Rust".to_string() },
//! ];
//!
//! assert_eq!(post_list, expected);
//! # }
//! ```
//!
//! If you're coming from other ORMs, you'll notice that this design is quite different from most.
//! There you would have an instance method on the parent, or have the children stored somewhere on
//! the posts. This design leads to many problems, including [N+1 query
//! bugs][load-your-entire-database-into-memory-lol], and runtime errors when accessing an
//! association that isn't there.
//!
//! [load-your-entire-database-into-memory-lol]: https://stackoverflow.com/q/97197/1254484
//!
//! In Diesel, data and its associations are considered to be separate. If you want to pass around
//! a user and all of its posts, that type is `(User, Vec<Post>)`.
//!
//! Next lets look at how to load the children for more than one parent record.
//! [`belonging_to`] can be used to load the data, but we'll also need to group it
//! with its parents. For this we use an additional method [`grouped_by`].
//!
//! [`grouped_by`]: GroupedBy::grouped_by
//! [`belonging_to`]: crate::query_dsl::BelongingToDsl::belonging_to
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! # use schema::{posts, users};
//! #
//! # #[derive(Identifiable, Queryable)]
//! # pub struct User {
//! #     id: i32,
//! #     name: String,
//! # }
//! #
//! # #[derive(Debug, PartialEq)]
//! # #[derive(Identifiable, Queryable, Associations)]
//! # #[diesel(belongs_to(User))]
//! # pub struct Post {
//! #     id: i32,
//! #     user_id: i32,
//! #     title: String,
//! # }
//! #
//! # fn main() {
//! #     run_test();
//! # }
//! #
//! # fn run_test() -> QueryResult<()> {
//! #     let connection = &mut establish_connection();
//! #     use self::users::dsl::*;
//! #     use self::posts::dsl::{posts, title};
//! let sean = users.filter(name.eq("Sean")).first::<User>(connection)?;
//! let tess = users.filter(name.eq("Tess")).first::<User>(connection)?;
//!
//! let seans_posts = Post::belonging_to(&sean)
//!     .select(title)
//!     .load::<String>(connection)?;
//! assert_eq!(vec!["My first post", "About Rust"], seans_posts);
//!
//! // A vec or slice can be passed as well
//! let more_posts = Post::belonging_to(&vec![sean, tess])
//!     .select(title)
//!     .load::<String>(connection)?;
//! assert_eq!(vec!["My first post", "About Rust", "My first post too"], more_posts);
//! #     Ok(())
//! # }
//! ```
//!
//! Typically you will want to group up the children with their parents.
//! In other ORMs, this is often called a `has_many` relationship.
//! Diesel provides support for doing this grouping, once the data has been
//! loaded.
//!
//! [`grouped_by`] is called on a `Vec<Child>` with a `&[Parent]`.
//! The return value will be `Vec<Vec<Child>>` indexed to match their parent.
//! Or to put it another way, the returned data can be passed to `zip`,
//! and it will be combined with its parent.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! # use schema::{posts, users};
//! #
//! # #[derive(Identifiable, Queryable, PartialEq, Debug)]
//! # pub struct User {
//! #     id: i32,
//! #     name: String,
//! # }
//! #
//! # #[derive(Debug, PartialEq)]
//! # #[derive(Identifiable, Queryable, Associations)]
//! # #[diesel(belongs_to(User))]
//! # pub struct Post {
//! #     id: i32,
//! #     user_id: i32,
//! #     title: String,
//! # }
//! #
//! # fn main() {
//! #     run_test();
//! # }
//! #
//! # fn run_test() -> QueryResult<()> {
//! #     let connection = &mut establish_connection();
//! let users = users::table.load::<User>(connection)?;
//! let posts = Post::belonging_to(&users)
//!     .load::<Post>(connection)?
//!     .grouped_by(&users);
//! let data = users.into_iter().zip(posts).collect::<Vec<_>>();
//!
//! let expected_data = vec![
//!     (
//!         User { id: 1, name: "Sean".into() },
//!         vec![
//!             Post { id: 1, user_id: 1, title: "My first post".into() },
//!             Post { id: 2, user_id: 1, title: "About Rust".into() },
//!         ],
//!     ),
//!     (
//!         User { id: 2, name: "Tess".into() },
//!         vec![
//!             Post { id: 3, user_id: 2, title: "My first post too".into() },
//!         ],
//!     ),
//! ];
//!
//! assert_eq!(expected_data, data);
//! #     Ok(())
//! # }
//! ```
//!
//! [`grouped_by`] can be called multiple times
//! if you have multiple children or grandchildren.
//!
//! For example, this code will load some users,
//! all of their posts,
//! and all of the comments on those posts.
//! Explicit type annotations have been added
//! to make each line a bit more clear.
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! # use schema::{users, posts, comments};
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable)]
//! # pub struct User {
//! #     id: i32,
//! #     name: String,
//! # }
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable, Associations)]
//! # #[diesel(belongs_to(User))]
//! # pub struct Post {
//! #     id: i32,
//! #     user_id: i32,
//! #     title: String,
//! # }
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable, Associations)]
//! # #[diesel(belongs_to(Post))]
//! # pub struct Comment {
//! #     id: i32,
//! #     post_id: i32,
//! #     body: String,
//! # }
//! #
//! # fn main() {
//! #   let connection = &mut establish_connection();
//! #
//! let users: Vec<User> = users::table.load::<User>(connection)
//!     .expect("error loading users");
//! let posts: Vec<Post> = Post::belonging_to(&users)
//!     .load::<Post>(connection)
//!     .expect("error loading posts");
//! let comments: Vec<Comment> = Comment::belonging_to(&posts)
//!     .load::<Comment>(connection)
//!     .expect("Error loading comments");
//! let grouped_comments: Vec<Vec<Comment>> = comments.grouped_by(&posts);
//! let posts_and_comments: Vec<Vec<(Post, Vec<Comment>)>> = posts
//!     .into_iter()
//!     .zip(grouped_comments)
//!     .grouped_by(&users);
//! let result: Vec<(User, Vec<(Post, Vec<Comment>)>)> = users
//!     .into_iter()
//!     .zip(posts_and_comments)
//!     .collect();
//! let expected = vec![
//!     (
//!         User { id: 1, name: "Sean".to_string() },
//!         vec![
//!             (
//!                 Post { id: 1, user_id: 1, title: "My first post".to_string() },
//!                 vec![ Comment { id: 1, post_id: 1, body: "Great post".to_string() } ]
//!             ),
//!             (
//!                 Post { id: 2, user_id: 1, title: "About Rust".to_string() },
//!                 vec![
//!                     Comment { id: 2, post_id: 2, body: "Yay! I am learning Rust".to_string() }
//!                 ]
//!
//!             )
//!         ]
//!     ),
//!     (
//!         User { id: 2, name: "Tess".to_string() },
//!         vec![
//!             (
//!                 Post { id: 3, user_id: 2, title: "My first post too".to_string() },
//!                 vec![ Comment { id: 3, post_id: 3, body: "I enjoyed your post".to_string() } ]
//!             )
//!         ]
//!     )
//! ];
//!
//! assert_eq!(result, expected);
//! # }
//! ```
//!
//! And that's it.
//! It may seem odd to have load, group, and zip be explicit separate steps
//! if you are coming from another ORM.
//! However, the goal is to provide simple building blocks which can
//! be used to construct the complex behavior applications need.
mod belongs_to;

use std::hash::Hash;

use crate::query_source::Table;

pub use self::belongs_to::{BelongsTo, GroupedBy};

#[doc(inline)]
pub use diesel_derives::Associations;

/// This trait indicates that a struct is associated with a single database table.
///
/// This trait is implemented by structs which implement `Identifiable`,
/// as well as database tables themselves.
pub trait HasTable {
    /// The table this type is associated with.
    type Table: Table;

    /// Returns the table this type is associated with.
    fn table() -> Self::Table;
}

impl<'a, T: HasTable> HasTable for &'a T {
    type Table = T::Table;

    fn table() -> Self::Table {
        T::table()
    }
}

/// This trait indicates that a struct represents a single row in a database table.
///
/// This must be implemented to use associations.
/// Additionally, implementing this trait allows you to pass your struct to `update`
/// (`update(&your_struct)` is equivalent to
/// `update(YourStruct::table().find(&your_struct.primary_key())`).
///
/// This trait is usually implemented on a reference to a struct,
/// not on the struct itself. It can be [derived](derive@Identifiable).
///
pub trait Identifiable: HasTable {
    /// The type of this struct's identifier.
    ///
    /// For single-field primary keys, this is typically `&'a i32`, or `&'a String`
    /// For composite primary keys, this is typically `(&'a i32, &'a i32)`
    /// or `(&'a String, &'a String)`, etc.
    type Id: Hash + Eq;

    /// Returns the identifier for this record.
    ///
    /// This takes `self` by value, not reference.
    /// This is because composite primary keys
    /// are typically stored as multiple fields.
    /// We could not return `&(String, String)` if each string is a separate field.
    ///
    /// Because of Rust's rules about specifying lifetimes,
    /// this means that `Identifiable` is usually implemented on references
    /// so that we have a lifetime to use for `Id`.
    fn id(self) -> Self::Id;
}

#[doc(inline)]
pub use diesel_derives::Identifiable;
