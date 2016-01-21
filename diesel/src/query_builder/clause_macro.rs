macro_rules! simple_clause {
    ($no_clause:ident, $clause:ident, $sql:expr) => {
        use super::{QueryFragment, QueryBuilder, BuildQueryResult};

        #[derive(Debug, Clone, Copy)]
        pub struct $no_clause;

        impl QueryFragment for $no_clause {
            fn to_sql(&self, _out: &mut QueryBuilder) -> BuildQueryResult {
                Ok(())
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct $clause<Expr>(pub Expr);

        impl<Expr> QueryFragment for $clause<Expr> where
            Expr: QueryFragment,
        {
            fn to_sql(&self, out: &mut QueryBuilder) -> BuildQueryResult {
                out.push_sql($sql);
                self.0.to_sql(out)
            }
        }
    }
}
