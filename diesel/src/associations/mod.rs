//! Traits related to relationships between multiple tables.
//!
//! **Note: This feature is under active development, and we are seeking feedback on the APIs that
//! have been released. Please feel free to [open issues][open-issue], or join [our chat][gitter]
//! to provide feedback.**
//!
//! [open-issue]: https://github.com/diesel-rs/diesel/issues/new
//! [gitter]: https://gitter.im/diesel-rs/diesel
//!
//! Associations in Diesel are unidirectional and focus on the child-to-parent
//! relationship.
//!
//! **Child** refers to the *many* part of a *one to many* relationship and has *one parent*.
//! **Parent** refers to the *one* part of a *one to many* relationship and can *have many
//! children*.
//! In the following relationship for our code examples, User has many Posts,
//! so User is the parent and Posts are children.
//!
//! In order to declare this relationship, you must annotate your child struct
//! with `#[derive(Associations)]` and `#[belongs_to(ParentStructName)]`.
//! `#[belongs_to]` is given the name of the struct that represents the parent. Both parent and
//! child types must implement the [`Identifiable`][identifiable] trait.
//! The struct or table referenced in your association has to be in scope,
//! so you'll need `use schema::posts` or similar to bring the
//! table into scope.
//!
//! ```rust
//! # #[macro_use] extern crate diesel;
//! # include!("../doctest_setup.rs");
//! // Brings User and Post tables into scope.
//! // Otherwise `table!` definitions would need to be here.
//! use schema::{posts, users};
//!
//! #[derive(Identifiable, Queryable)]
//! pub struct User {
//!     id: i32,
//!     name: String,
//! }
//!
//! # #[derive(Debug, PartialEq)]
//! #[derive(Identifiable, Queryable, Associations)]
//! #[belongs_to(User)]
//! pub struct Post {
//!     id: i32,
//!     user_id: i32,
//!     title: String,
//! }
//!
//! # fn main() {
//! # let connection = establish_connection();
//! # use users::dsl::*;
//! let user = users.find(2).get_result::<User>(&connection).unwrap();
//! let posts = Post::belonging_to(&user)
//!     .load::<Post>(&connection);
//!
//! assert_eq!(posts,
//!     Ok(vec![Post { id: 3, user_id: 2, title: "My first post too".to_owned() }])
//! );
//! # }
//! ```
//!
//! [Identifiable]: trait.Identifiable.html
//!
//! If the name of your foreign key doesn't follow the convention `tablename_id`, you can specify a
//! custom one to `#[belongs_to]` by adding a `foreign_key` argument to the
//! attribute like so `#[belongs_to(Foo, foreign_key="mykey")]`.
//!
//! `Associations` are separate from joins in Diesel. Please see docs on [`JoinDsl`]
//! to learn more about joining tables.
//!
//! [`JoinDsl`]: ../query_dsl/trait.JoinDsl.html
//!
//! You can load the children for a single parent or multiple parents (`Vec<ParentType>`) using the
//! [`belonging_to()`][belonging-to] method.
//! This amounts to the `SQL` statements `SELECT * FROM posts WHERE posts.user_id = $1`
//! or `SELECT * FROM posts WHERE posts.user_id IN ($1, $2, etc...)`
//!
//! [belonging-to]: ../query_dsl/trait.BelongingToDsl.html#tymethod.belonging_to
//!
//! ```rust
//! # #[macro_use] extern crate diesel;
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
//! # #[belongs_to(User)]
//! # pub struct Post {
//! #     id: i32,
//! #     user_id: i32,
//! #     title: String,
//! # }
//! #
//! # fn main() {
//! #   use users::dsl::*;
//! #   let connection = establish_connection();
//! #
//! // Loading the posts for a single parent user.
//! let user = users.load::<User>(&connection).expect("Error loading user");
//! let post_list = Post::belonging_to(&user)
//!     .load::<Post>(&connection)
//!     .expect("Error loading posts");
//! let expected = vec![
//!     Post { id: 1, user_id: 1, title: "My first post".to_string() },
//!     Post { id: 2, user_id: 1, title: "About Rust".to_string() },
//!     Post { id: 3, user_id: 2, title: "My first post too".to_string() }
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
//! [`belonging_to`][belonging-to] can be used to load all of the child data, but we'll also need to group the
//! child records with their parents.
//! For this we use an additional method [`grouped_by`][grouped-by]
//!
//! [grouped-by]: trait.GroupedBy.html#tymethod.grouped_by
//!
//! ```rust
//! # #[macro_use] extern crate diesel;
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
//! # #[belongs_to(User)]
//! # pub struct Post {
//! #     id: i32,
//! #     user_id: i32,
//! #     title: String,
//! # }
//! #
//! # fn main() {
//! #   use users::dsl::*;
//! #   let connection = establish_connection();
//! #
//! let users_vec = users
//!     .load::<User>(&connection)
//!     .expect("Error loading user");
//! let posts_vec = Post::belonging_to(&users_vec)
//!     .load::<Post>(&connection)
//!     .expect("Error loading posts");
//! let grouped_posts = posts_vec.grouped_by(&users_vec);
//! let result: Vec<(User, Vec<Post>)> = users_vec.into_iter().zip(grouped_posts).collect();
//! let expected = vec![
//!     (
//!         User { id: 1, name: "Sean".to_string() },
//!         vec![
//!             Post { id: 1, user_id: 1, title: "My first post".to_string() },
//!             Post { id: 2, user_id: 1, title: "About Rust".to_string() },
//!         ]
//!     ),
//!     (
//!         User { id: 2, name: "Tess".to_string() },
//!         vec![
//!             Post { id: 3, user_id: 2, title: "My first post too".to_string() }
//!         ]
//!     )
//! ];
//!
//! assert_eq!(result, expected);
//! # }
//! ```
//! [`grouped_by`][grouped-by] is called on a `Vec<Child>` with their `&Vec<Parent>` and returns a
//! `Vec<Vec<Child>>` where the index of the children matches the index of the parent they belong
//! to. Or to put it another way, it returns them in an order ready to be `zip`ed with the parents. You
//! can do this multiple times. For example, if you wanted to load the comments for all the posts
//! as well, you could do this: (explicit type annotations have been added for documentation
//! purposes)
//!
//! ```rust
//! # #[macro_use] extern crate diesel;
//! # include!("../doctest_setup.rs");
//! # use schema::users;
//! # use schema::posts;
//! # use schema::comments;
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable)]
//! # pub struct User {
//! #     id: i32,
//! #     name: String,
//! # }
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable, Associations)]
//! # #[belongs_to(User)]
//! # pub struct Post {
//! #     id: i32,
//! #     user_id: i32,
//! #     title: String,
//! # }
//! #
//! # #[derive(Debug, PartialEq, Identifiable, Queryable, Associations)]
//! # #[belongs_to(Post)]
//! # pub struct Comment {
//! #     id: i32,
//! #     post_id: i32,
//! #     body: String,
//! # }
//! #
//! # fn main() {
//! #   use users::dsl::*;
//! #   let connection = establish_connection();
//! #
//! let users_vec: Vec<User> = users.load::<User>(&connection)
//!     .expect("error loading users");
//! let posts_vec: Vec<Post> = Post::belonging_to(&users_vec)
//!     .load::<Post>(&connection)
//!     .expect("error loading posts");
//! let comments_vec: Vec<Comment> = Comment::belonging_to(&posts_vec)
//!     .load::<Comment>(&connection)
//!     .expect("Error loading comments");
//! let grouped_comments: Vec<Vec<Comment>> = comments_vec.grouped_by(&posts_vec);
//! let posts_and_comments: Vec<Vec<(Post, Vec<Comment>)>> = posts_vec
//!     .into_iter()
//!     .zip(grouped_comments)
//!     .grouped_by(&users_vec);
//! let result: Vec<(User, Vec<(Post, Vec<Comment>)>)> = users_vec
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
//!
//!
//! ```
mod belongs_to;

use std::hash::Hash;

use query_source::Table;

pub use self::belongs_to::{BelongsTo, GroupedBy};

pub trait HasTable {
    type Table: Table;

    fn table() -> Self::Table;
}

impl<'a, T: HasTable> HasTable for &'a T {
    type Table = T::Table;

    fn table() -> Self::Table {
        T::table()
    }
}

/// Represents a struct which can be identified on a single table in the
/// database. This must be implemented to use associations, and some features of
/// updating. This trait is usually implemented on a reference to a struct, not
/// the struct itself.
///
/// ### Deriving
///
/// This trait can be automatically derived by adding `#[derive(Identifiable)]`
/// to your struct. The primary key will be assumed to be a field and
/// column called `id`. If it's not, you can annotate your structure with `#[primary_key(your_id)]`
/// or `#[primary_key(your_id, second_id)]`. By default the table will be assumed to be the plural
/// form of the struct name (using *very* dumb pluralization -- it just adds an `s` at the end). If
/// your table name differs from that convention, or requires complex pluralization, it can be
/// specified using `#[table_name = "some_table_name"]`. The inferred table name is considered
/// public API and will never change without a major version bump.
pub trait Identifiable: HasTable {
    type Id: Hash + Eq;

    fn id(self) -> Self::Id;
}
