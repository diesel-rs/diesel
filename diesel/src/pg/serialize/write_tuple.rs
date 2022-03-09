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
/// ```
/// # #[cfg(feature = "postgres")]
/// # mod the_impl {
/// #     use diesel::prelude::*;
/// #     use diesel::pg::Pg;
/// #     use diesel::serialize::{self, ToSql, Output, WriteTuple};
/// #     use diesel::sql_types::{Integer, Text, SqlType};
/// #
///     #[derive(SqlType)]
///     #[diesel(postgres_type(name = "my_type"))]
///     struct MyType;
///
///     #[derive(Debug)]
///     struct MyStruct<'a>(i32, &'a str);
///
///     impl<'a> ToSql<MyType, Pg> for MyStruct<'a> {
///         fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
///             WriteTuple::<(Integer, Text)>::write_tuple(
///                 &(self.0, self.1),
///                 &mut out.reborrow(),
///             )
///         }
///     }
/// # }
/// # fn main() {}
/// ```
#[cfg(feature = "postgres_backend")]
pub trait WriteTuple<ST> {
    /// See trait documentation.
    fn write_tuple(&self, out: &mut Output<'_, '_, Pg>) -> serialize::Result;
}
