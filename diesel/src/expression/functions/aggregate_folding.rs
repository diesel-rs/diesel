use backend::Backend;
use expression::{Expression, SelectableExpression};
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
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
                out.push_sql(concat!($operator, "("));
                try!(self.target.to_sql(out));
                out.push_sql(")");
                Ok(())
            }

            fn collect_binds(&self, out: &mut DB::BindCollector) -> QueryResult<()> {
                try!(self.target.collect_binds(out));
                Ok(())
            }
        }

        impl<ST, T, QS> SelectableExpression<QS> for $type_name<T> where
            ST: Foldable,
            T: Expression<SqlType=ST>,
        {
        }
    }
}

fold_function!(sum, Sum, "SUM",
"Represents a SQL `SUM` function. This function can only take types which are
Foldable.");

fold_function!(avg, Avg, "AVG",
"Represents a SQL `AVG` function. This function can only take types which are
Foldable.");
