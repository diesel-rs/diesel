// FIXME: Replace these examples with executable tests
//! Traits related to relationships between multiple tables.
//!
//! **Note: This feature is under active development, and we are seeking feedback on the APIs that
//! have been released. Please feel free to [open issues][open-issue], or join [our chat][gitter]
//! to provide feedback.**
//!
//! [open-issue]: https://github.com/diesel-rs/diesel/issues/new
//! [gitter]: https://gitter.im/diesel-rs/diesel
//!
//! Note: This guide is written assuming the usage of `diesel_codegen` or `diesel_codegen_syntex`.
//! If you are using [`custom_derive`][custom-derive] instead, you will need to replace
//! `#[belongs_to(Foo)]` with `#[derive(BelongsTo(Foo, foreign_key = foo_id))]` and
//! `#[has_many(bars)]` with `#[derive(HasMany(bars, foreign_key = bar_id))]`.
//!
//! [custom-derive]: https://crates.io/crates/custom_derive
//!
//! Associations in Diesel are bidirectional, but primarily focus on the child-to-parent
//! relationship. You can declare an association between two records with `#[has_many]` and
//! `#[belongs_to]`.
//!
//! ```ignore
//! #[derive(Identifiable, Queryable)]
//! #[has_many(posts)]
//! pub struct User {
//!     id: i32,
//!     name: String,
//! }
//!
//! #[derive(Identifiable, Queryable)]
//! #[belongs_to(User)]
//! pub struct Post {
//!     id: i32,
//!     user_id: i32,
//!     title: String,
//! }
//! ```
//!
//! `#[has_many]` should be passed the name of the table that the children will be loaded from,
//! while `#[belongs_to]` is given the name of the struct that represents the parent. Both types
//! must implement the [`Identifiable`][identifiable] trait.
//!
//! [Identifiable]: associations/trait.Identifiable.html
//!
//! `#[has_many]` actually has no behavior on its own. It only enables joining between the two
//! tables. If you are only accessing data through the [`belonging_to`][belonging-to] method, you
//! only need to define the `#[belongs_to]` side.
//!
//! Once the associations are defined, you can join between the two tables using the
//! [`inner_join`][inner-join] or [`left_outer_join`][left-outer-join].
//!
//! [inner-join]: query_source/trait.Table.html#method.inner_join
//! [left-outer-join]: query_source/trait.Table.html#method.left_outer_join
//!
//! ```ignore
//! let data: Vec<(User, Post)> = users::table.inner_join(posts::table).load(&connection);
//! ```
//!
//! Note: Due to language limitations, only two tables can be joined per query. This will change in
//! the future.
//!
//! Typically however, queries are loaded in multiple queries. For most datasets, the reduced
//! amount of duplicated information sent over the wire saves more time than the extra round trip
//! costs us. You can load the children for a single parent using the
//! [`belonging_to`][belonging-to]
//!
//! [belonging-to]: prelude/trait.BelongingToDsl.html#tymethod.belonging_to
//!
//! ```ignore
//! let user = try!(users::find(1).first(&connection));
//! let posts = Post::belonging_to(&user).load(&connection);
//! ```
//!
//! If you're coming from other ORMs, you'll notice that this design is quite different from most.
//! There you would have an instance method on the parent, or have the children stored somewhere on
//! the posts. This design leads to many problems, including [N+1 query
//! bugs][load-your-entire-database-into-memory-lol], and runtime errors when accessing an
//! association that isn't there.
//!
//! [load-your-entire-database-into-memory-lol]: http://stackoverflow.com/q/97197/1254484
//!
//! In Diesel, data and its associations are considered to be separate. If you want to pass around
//! a user and all of its posts, that type is `(User, Vec<Post>)`.
//!
//! Next lets look at how to load the children for more than one parent record.
//! [`belonging_to`][belonging-to] can be used to load the data, but we'll also need to group it
//! with its parents. For this we use an additional method [`grouped_by`][grouped-by]
//!
//! [grouped-by]: associations/trait.GroupedBy.html#tymethod.grouped_by
//!
//! ```ignore
//! fn first_twenty_users_and_their_posts(conn: &PgConnection) -> QueryResult<Vec<(User, Vec<Post>)>> {
//!     let users = try!(users::limit(20).load::<User>(conn));
//!     let posts = try!(Post::belonging_to(&users).load::<Post>(conn));
//!     let grouped_posts = posts.grouped_by(&users);
//!     users.into_iter().zip(grouped_posts).collect()
//! }
//! ```
//!
//! [`grouped_by`][grouped-by] takes a `Vec<Child>` and a `Vec<Parent>` and returns a
//! `Vec<Vec<Child>>` where the index of the children matches the index of the parent they belong
//! to. Or to put it another way, it returns them in an order ready to be `zip`ed with the parents. You
//! can do this multiple times. For example, if you wanted to load the comments for all the posts
//! as well, you could do this: (explicit type annotations have been added for documentation
//! purposes)
//!
//! ```ignore
//! let posts: Vec<Post> = try!(Post::belonging_to(&users).load());
//! let comments: Vec<Comment> = try!(Comment::belonging_to(&posts).load());
//! let comments: Vec<Vec<Comment>> = comments.grouped_by(&posts);
//! let posts_and_comments: Vec<Vec<(Post, Vec<Comment>)>> = posts.into_iter().zip(comments).grouped_by(&users);
//! let result: Vec<(User, Vec<(Post, Vec<Comment>)>)> = users.into_iter().zip(posts_and_comments).collect();
//! ```
//!
//! And that's it. This module will be expanded in the future with more complex joins, and the
//! ability to define "through" associations (e.g. load all the comments left on any posts written
//! by a user in a single query). However, the goal is to provide simple building blocks which can
//! be used to construct the complex behavior applications need.
mod belongs_to;

use std::hash::Hash;

use query_dsl::FindDsl;
use query_source::Table;

pub use self::belongs_to::{BelongsTo, GroupedBy};

pub trait Identifiable {
    type Id: Hash + Eq;
    type Table: Table + for<'a> FindDsl<&'a Self::Id>;

    fn table() -> Self::Table;
    fn id(&self) -> &Self::Id;
}
