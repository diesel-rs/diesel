/// Constructs a query that finds record(s) based on directional association with other record(s).
///
/// # Example
///
/// ```rust
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
/// #     use self::users::dsl::*;
/// #     use self::posts::dsl::{posts, title};
/// let sean = users.filter(name.eq("Sean")).first::<User>(connection)?;
/// let tess = users.filter(name.eq("Tess")).first::<User>(connection)?;
///
/// let seans_posts = Post::belonging_to(&sean)
///     .select(title)
///     .load::<String>(connection)?;
/// assert_eq!(vec!["My first post", "About Rust"], seans_posts);
///
/// // A vec or slice can be passed as well
/// let more_posts = Post::belonging_to(&vec![sean, tess])
///     .select(title)
///     .load::<String>(connection)?;
/// assert_eq!(vec!["My first post", "About Rust", "My first post too"], more_posts);
/// #     Ok(())
/// # }
/// ```
pub trait BelongingToDsl<T> {
    /// The query returned by `belonging_to`
    type Output;

    /// Get the record(s) belonging to record(s) `other`
    fn belonging_to(other: T) -> Self::Output;
}
