use std::io::Write;

use crate::pg::Pg;
use crate::serialize::{self, Output};

/// Helper trait for writing tuples as named composite types
///
/// This trait is essentially `ToSql<Record<ST>>` for tuples.
/// While we can provide a valid body of `to_sql`,
/// PostgreSQL doesn't allow the use of bind parameters for unnamed composite types.
/// For this reason, we avoid implementing `ToSql` directly.
///
/// This trait can be used by `ToSql` impls of named composite types.
///
/// # Example
///
/// ```no_run
/// # #[macro_use]
/// # extern crate diesel;
/// #
/// # #[cfg(feature = "postgres")]
/// # mod the_impl {
/// #     use diesel::pg::Pg;
/// #     use diesel::serialize::{self, ToSql, Output, WriteTuple};
/// #     use diesel::sql_types::{Integer, Text};
/// #     use std::io::Write;
/// #
///     #[derive(SqlType)]
///     #[postgres(type_name = "my_type")]
///     struct MyType;
///
///     #[derive(Debug)]
///     struct MyStruct<'a>(i32, &'a str);
///
///     impl<'a> ToSql<MyType, Pg> for MyStruct<'a> {
///         fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
///             WriteTuple::<(Integer, Text)>::write_tuple(
///                 &(self.0, self.1),
///                 out,
///             )
///         }
///     }
/// # }
/// # fn main() {}
/// ```
pub trait WriteTuple<ST> {
    /// See trait documentation.
    fn write_tuple<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result;
}
