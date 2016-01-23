macro_rules! simple_clause {
    ($no_clause:ident, $clause:ident, $sql:expr) => {
        use backend::Backend;
        use super::{QueryFragment, QueryBuilder, BuildQueryResult};

        #[derive(Debug, Clone, Copy)]
        pub struct $no_clause;

        impl<DB: Backend> QueryFragment<DB> for $no_clause {
            fn to_sql(&self, _out: &mut DB::QueryBuilder) -> BuildQueryResult {
                Ok(())
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $clause<Expr>(pub Expr);

        impl<Expr, DB> QueryFragment<DB> for $clause<Expr> where
            DB: Backend,
            Expr: QueryFragment<DB>,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
                out.push_sql($sql);
                self.0.to_sql(out)
            }
        }
    }
}
