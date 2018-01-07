use backend::Backend;
use expression::Expression;
use query_builder::*;
use result::QueryResult;
use types::Foldable;

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

        #[derive(Debug, Clone, Copy, QueryId)]
        #[doc(hidden)]
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
# use diesel::dsl::*;
#
# fn main() {
#     use schema::animals::dsl::*;
#     let connection = establish_connection();
assert_eq!(Ok(Some(12i64)), animals.select(sum(legs)).first(&connection));
# }
"
);

fold_function!(
    avg,
    Avg,
    "AVG",
    r#"Represents a SQL `AVG` function. This function can only take types which are
Foldable.

# Examples

```rust
# #[macro_use] extern crate diesel;
# include!("../../doctest_setup.rs");
# use diesel::dsl::*;
# #[cfg(feature = "bigdecimal")]
# extern crate bigdecimal;
#
# fn main() {
#     run_test().unwrap();
# }
#
# table! {
#     numbers (number) {
#         number -> Integer,
#     }
# }
#
# #[cfg(all(feature = "numeric", any(feature = "postgres", not(feature = "sqlite"))))]
# fn run_test() -> QueryResult<()> {
#     use bigdecimal::BigDecimal;
#     use numbers::dsl::*;
#     let conn = establish_connection();
#     conn.execute("DROP TABLE IF EXISTS numbers")?;
#     conn.execute("CREATE TABLE numbers (number INTEGER)")?;
diesel::insert_into(numbers)
    .values(&vec![number.eq(1), number.eq(2)])
    .execute(&conn)?;
let average = numbers.select(avg(number)).get_result(&conn)?;
let expected = "1.5".parse::<BigDecimal>().unwrap();
assert_eq!(Some(expected), average);
#     Ok(())
# }
#
# #[cfg(not(all(feature = "numeric", any(feature = "postgres", not(feature = "sqlite")))))]
# fn run_test() -> QueryResult<()> {
#     Ok(())
# }
```
"#
);
