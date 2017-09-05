use backend::Backend;
use expression::Expression;
use query_builder::*;
use result::QueryResult;
use types::{Foldable, HasSqlType};

macro_rules! fold_function {
    ($fn_name:ident, $type_name:ident, $operator:expr, $docs:expr) => {
        #[doc=$docs]
        pub fn $fn_name<ST, T>(t: T) -> $type_name<T> where
            ST: Foldable,
            T: Expression<SqlType=ST>,
        {
            $type_name {
                target: t,
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $type_name<T> {
            target: T,
        }

        impl<ST, T> Expression for $type_name<T> where
            ST: Foldable,
            T: Expression<SqlType=ST>
        {
            type SqlType = <<T as Expression>::SqlType as Foldable>::$type_name;
        }

        impl<T, DB> QueryFragment<DB> for $type_name<T> where
            T: Expression + QueryFragment<DB>,
            DB: Backend + HasSqlType<T::SqlType>,
        {
            fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
                out.push_sql(concat!($operator, "("));
                self.target.walk_ast(out.reborrow())?;
                out.push_sql(")");
                Ok(())
            }
        }

        impl_query_id!($type_name<T>);
        impl_selectable_expression!($type_name<T>);
    }
}

fold_function!(
    sum,
    Sum,
    "SUM",
    "Represents a SQL `SUM` function. This function can only take types which are
Foldable.

# Examples

```rust
# #[macro_use] extern crate diesel;
# include!(\"../../doctest_setup.rs\");
# use diesel::expression::dsl::*;
#
# table! {
#     users {
#         id -> Integer,
#         name -> VarChar,
#     }
# }
#
# fn main() {
#     use self::animals::dsl::*;
#     let connection = establish_connection();
assert_eq!(Ok(Some(12i64)), animals.select(sum(legs)).first(&connection));
# }
"
);

fold_function!(
    avg,
    Avg,
    "AVG",
    "Represents a SQL `AVG` function. This function can only take types which are
Foldable.

# Examples

```rust
# #[macro_use] extern crate diesel;
# include!(\"../../doctest_setup.rs\");
# use diesel::expression::dsl::*;
#
# table! {
#     users {
#         id -> Integer,
#         name -> VarChar,
#     }
# }
#
# fn main() {
#     use self::animals::dsl::*;
#     let connection = establish_connection();
// assert_eq!(Ok(Some(6f64)), animals.select(avg(legs)).first(&connection));
// TODO: There doesn't currently seem to be a way to use avg with integers, since
// they return a `Numeric` which doesn't have a corresponding Rust type.
# }
```
"
);
