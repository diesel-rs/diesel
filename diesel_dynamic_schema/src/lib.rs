//! # Diesel dynamic schema
//!
//! Diesel is an ORM and query builder designed to reduce
//! the boilerplate for database interactions.
//!
//! If this is your first time reading about Diesel, then
//! we recommend you start with the [getting started guide].
//! We also have [many other long form guides].
//!
//! [getting started guide]: https://diesel.rs/guides/getting-started/
//! [many other long form guides]: https://diesel.rs/guides
//!
//! Diesel is built to provide strong compile time guarantees that your
//! queries are valid. To do this, it needs to represent your schema
//! at compile time. However, there are some times where you don't
//! actually know the schema you're interacting with until runtime.
//!
//! This crate provides tools to work with those cases, while still being
//! able to use Diesel's query builder. Keep in mind that many compile time
//! guarantees are lost. We cannot verify that the tables/columns you ask
//! for actually exist, or that the types you state are correct.
//!
//! # Getting Started
//!
//! The `table` function is used to create a new Diesel table.
//! Note that you must always provide an explicit select clause
//! when using this crate.
//!
//! ```rust
//! # mod connection_setup {
//! #     include!("../tests/connection_setup.rs");
//! # }
//! # use connection_setup::establish_connection;
//! # use diesel::prelude::*;
//! # use diesel::sql_types::{Integer, Text};
//! # use diesel_dynamic_schema::table;
//! # use diesel::dsl::sql_query;
//! #
//! #
//! # fn result_main() -> QueryResult<()> {
//! #
//! # let conn = &mut establish_connection();
//! #
//! # // Create some example data by using typical SQL statements.
//! # connection_setup::create_user_table(conn);
//! # sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").execute(conn)?;
//! #
//! // Use diesel-dynamic-schema to create a table and columns.
//! let users = table("users");
//! let id = users.column::<Integer, _>("id");
//! let name = users.column::<Text, _>("name");
//!
//! // Now you can use typical Diesel syntax; see the Diesel docs for more.
//! let results = users
//!     .select((id, name))
//!     .filter(name.eq("Sean"))
//!     .load::<(i32, String)>(conn)?;
//!
//! # assert_eq!(results.len(), 1);
//! # assert_eq!(results[0].1, "Sean");
//! #
//! for (id, name) in results {
//!     println!("id:{} name:{}", id, name);
//! }
//! # Ok(())
//! # }
//! # result_main().unwrap()
//! ```
//!
//! See the `/examples` directory for runnable code examples.
//!
//! ## Getting help
//!
//! If you run into problems, Diesel has a very active Gitter room.
//! You can come ask for help at
//! [gitter.im/diesel-rs/diesel](https://gitter.im/diesel-rs/diesel)

// Built-in Lints
#![warn(missing_docs)]
#![deny(warnings)]

mod column;
mod dummy_expression;
mod dynamic_select;
pub mod dynamic_value;
mod schema;
mod table;

/// A database table column.
pub use column::Column;

/// A database schema.
pub use schema::Schema;

/// A database table.
pub use table::Table;

#[doc(inline)]
pub use self::dynamic_select::DynamicSelectClause;

/// Create a new [`Table`] with the given name.
///
/// # Example
///
/// ```
/// use diesel_dynamic_schema::table;
///
/// let users = table("users");
/// ```
pub fn table<T>(name: T) -> Table<T> {
    Table::new(name)
}

/// Create a new [`Schema`] with the given name.
///
/// # Example
///
/// ```
/// use diesel_dynamic_schema::schema;
///
/// let schema = schema("users");
/// ```
pub fn schema<T>(name: T) -> Schema<T> {
    Schema::new(name)
}
