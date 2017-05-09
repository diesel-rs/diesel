macro_rules! simple_clause {
    ($no_clause:ident, $clause:ident, $sql:expr) => {
        simple_clause!($no_clause, $clause, $sql, backend_bounds = );
    };

    ($no_clause:ident, $clause:ident, $sql:expr, backend_bounds = $($backend_bounds:ident),*) => {
        use backend::Backend;
        use result::QueryResult;
        use super::{QueryFragment, QueryBuilder, BuildQueryResult, AstPass};

        #[derive(Debug, Clone, Copy)]
        pub struct $no_clause;

        impl<DB: Backend> QueryFragment<DB> for $no_clause {
            fn to_sql(&self, _out: &mut DB::QueryBuilder) -> BuildQueryResult {
                Ok(())
            }

            fn walk_ast(&self, _: &mut AstPass<DB>) -> QueryResult<()> {
                Ok(())
            }
        }

        impl_query_id!($no_clause);

        #[derive(Debug, Clone, Copy)]
        pub struct $clause<Expr>(pub Expr);

        impl<Expr, DB> QueryFragment<DB> for $clause<Expr> where
            DB: Backend $(+ $backend_bounds)*,
            Expr: QueryFragment<DB>,
        {
            fn to_sql(&self, out: &mut DB::QueryBuilder) -> BuildQueryResult {
                out.push_sql($sql);
                self.0.to_sql(out)
            }

            fn walk_ast(&self, pass: &mut AstPass<DB>) -> QueryResult<()> {
                self.0.walk_ast(pass)
            }
        }

        impl_query_id!($clause<Expr>);
    }
}
