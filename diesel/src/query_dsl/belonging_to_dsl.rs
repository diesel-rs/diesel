/// Constructs a query that finds record(s) based on directional association with other record(s).
///
/// # Example
///
/// ```rust
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_codegen;
/// # include!("../doctest_setup.rs");
/// # use schema::{posts, users};
/// #
/// # #[derive(Identifiable, Queryable)]
/// # pub struct User {
/// #     id: i32,
/// #     name: String,
/// # }
/// #
/// # #[derive(Debug, PartialEq)]
/// # #[derive(Identifiable, Queryable, Associations)]
/// # #[belongs_to(User)]
/// # pub struct Post {
/// #     id: i32,
/// #     user_id: i32,
/// #     title: String,
/// # }
/// #
/// # fn main() {
/// # let connection = establish_connection();
/// # use users::dsl::*;
/// # let user = users.find(2).get_result::<User>(&connection).unwrap();
/// let posts = Post::belonging_to(&user)
/// #    .load::<Post>(&connection);
/// #
/// # assert_eq!(posts,
/// #     Ok(vec![Post { id: 3, user_id: 2, title: "My first post too".to_owned() }])
/// # );
/// # }
/// ```
pub trait BelongingToDsl<T> {
    /// The query returned by `belonging_to`
    type Output;

    /// Get the record(s) belonging to record(s) `other`
    fn belonging_to(other: T) -> Self::Output;
}
