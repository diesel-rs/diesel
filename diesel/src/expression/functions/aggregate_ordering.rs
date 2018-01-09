use backend::Backend;
use expression::Expression;
use query_builder::*;
use result::QueryResult;
use types::{IntoNullable, SqlOrd};

macro_rules! ord_function {
    ($fn_name:ident, $type_name:ident, $operator:expr, $docs:expr) => {
        #[doc=$docs]
        pub fn $fn_name<ST, T>(t: T) -> $type_name<T> where
            ST: SqlOrd,
            T: Expression<SqlType=ST>,
        {
            $type_name {
                target: t,
            }
        }

        #[derive(Debug, Clone, Copy, QueryId)]
        #[doc(hidden)]
        pub struct $type_name<T> {
            target: T,
        }

        impl<T: Expression> Expression for $type_name<T> where
            T::SqlType: IntoNullable,
        {
            type SqlType = <T::SqlType as IntoNullable>::Nullable;
        }

        impl<T, DB> QueryFragment<DB> for $type_name<T> where
            T: Expression + QueryFragment<DB>,
            DB: Backend,
        {
            fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
                out.push_sql(concat!($operator, "("));
                self.target.walk_ast(out.reborrow())?;
                out.push_sql(")");
                Ok(())
            }
        }

        impl_selectable_expression!($type_name<T>);
    }
}

ord_function!(
    max,
    Max,
    "MAX",
    "Represents a SQL `MAX` function. This function can only take types which are
ordered.

# Examples

```rust
# #[macro_use] extern crate diesel;
# include!(\"../../doctest_setup.rs\");
# use diesel::dsl::*;
#
# fn main() {
#     use schema::animals::dsl::*;
#     let connection = establish_connection();
assert_eq!(Ok(Some(8)), animals.select(max(legs)).first(&connection));
# }
"
);

ord_function!(
    min,
    Min,
    "MIN",
    "Represents a SQL `MIN` function. This function can only take types which are
ordered.

# Examples

```rust
# #[macro_use] extern crate diesel;
# include!(\"../../doctest_setup.rs\");
# use diesel::dsl::*;
#
# fn main() {
#     use schema::animals::dsl::*;
#     let connection = establish_connection();
assert_eq!(Ok(Some(4)), animals.select(min(legs)).first(&connection));
# }
"
);
