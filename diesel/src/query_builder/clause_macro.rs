macro_rules! simple_clause {
    (
        $(#[$no_clause_meta: meta])*
        $no_clause:ident,
        $(#[$clause_meta: meta])*
        $clause:ident,
        $sql:expr
    ) => {
        simple_clause!(
            $(#[$no_clause_meta])*
            $no_clause,
            $(#[$clause_meta])*
            $clause,
            $sql,
            backend_bounds =
        );
    };

    (
        $(#[$no_clause_meta: meta])*
        $no_clause:ident,
        $(#[$clause_meta: meta])*
        $clause:ident,
        $sql:expr,
        backend_bounds = $($backend_bounds:ident),*
    ) => {
        use crate::backend::{Backend, DieselReserveSpecialization};
        use crate::result::QueryResult;
        use crate::query_builder::QueryId;
        use super::{QueryFragment, AstPass};

        $(#[$no_clause_meta])*
        #[derive(Debug, Clone, Copy, QueryId)]
        pub struct $no_clause;

        impl<DB> QueryFragment<DB> for $no_clause where
            DB: Backend + DieselReserveSpecialization,
        {
            fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()>
            {
                Ok(())
            }
        }

        $(#[$clause_meta])*
        #[derive(Debug, Clone, Copy, QueryId)]
        pub struct $clause<Expr>(pub Expr);

        impl<Expr, DB> QueryFragment<DB> for $clause<Expr> where
            DB: Backend + DieselReserveSpecialization $(+ $backend_bounds)*,
            Expr: QueryFragment<DB>,
        {
            fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()>
            {
                out.push_sql($sql);
                self.0.walk_ast(out.reborrow())?;
                Ok(())
            }
        }
    }
}
