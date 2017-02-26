use backend::Backend;
use expression::Expression;
use query_builder::*;
use result::QueryResult;
use types::{SqlOrd, HasSqlType, IntoNullable};

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

        #[derive(Debug, Clone, Copy)]
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

            fn is_safe_to_cache_prepared(&self) -> bool {
                self.target.is_safe_to_cache_prepared()
            }
        }

        impl_query_id!($type_name<T>);
        impl_selectable_expression!($type_name<T>);
    }
}

ord_function!(max, Max, "MAX",
"Represents a SQL `MAX` function. This function can only take types which are
ordered.");

ord_function!(min, Min, "MIN",
"Represents a SQL `MIN` function. This function can only take types which are
ordered.");
