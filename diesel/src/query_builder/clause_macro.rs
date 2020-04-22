macro_rules! simple_clause {
    (
        $(#[doc = $($no_clause_doc:tt)*])*
        $no_clause:ident,
        $(#[doc = $($clause_doc:tt)*])*
        $clause:ident,
        $sql:expr
    ) => {
        simple_clause!(
            $(#[doc = $($no_clause_doc)*])*
            $no_clause,
            $(#[doc = $($clause_doc)*])*
            $clause,
            $sql,
            backend_bounds =
        );
    };

    (
        $(#[doc = $($no_clause_doc:tt)*])*
        $no_clause:ident,
        $(#[doc = $($clause_doc:tt)*])*
        $clause:ident,
        $sql:expr,
        backend_bounds = $($backend_bounds:ident),*
    ) => {
        use crate::backend::Backend;
        use crate::result::QueryResult;
        use crate::query_builder::QueryId;
        use super::{QueryFragment, AstPass};

        $(#[doc = $($no_clause_doc)*])*
        #[derive(Debug, Clone, Copy, QueryId)]
        pub struct $no_clause;

        impl<DB: Backend> QueryFragment<DB> for $no_clause {
            fn walk_ast(&self, _: AstPass<DB>) -> QueryResult<()> {
                Ok(())
            }
        }

        $(#[doc = $($clause_doc)*])*
        #[derive(Debug, Clone, Copy, QueryId)]
        pub struct $clause<Expr>(pub Expr);

        impl<Expr, DB> QueryFragment<DB> for $clause<Expr> where
            DB: Backend $(+ $backend_bounds)*,
            Expr: QueryFragment<DB>,
        {
            fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
                out.push_sql($sql);
                self.0.walk_ast(out.reborrow())?;
                Ok(())
            }
        }
    }
}
