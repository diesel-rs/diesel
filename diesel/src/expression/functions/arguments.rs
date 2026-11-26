use crate::QueryResult;
use crate::backend::sql_dialect::sql_function_parameter_notation::{
    PositionalNotationOnly, PositionalOrNamedNotation,
};
use crate::backend::{Backend, SqlDialect};
use crate::query_builder::{AstPass, QueryFragment, QueryId};

/// Helper type to represent a parameter of a SQL function. Used to allow passing the parameter
/// by name or by position, depending on the value of `NAMED` and the
/// [`SqlDialect::SqlFunctionParameterNotation`] specialization type.
#[derive(Debug, QueryId, Copy, Clone)]
pub struct FunctionParameter<const NAMED: bool> {
    /// The name of the function parameter.
    pub name: &'static str,
}

impl<DB> QueryFragment<DB, PositionalNotationOnly> for FunctionParameter<false>
where
    DB: Backend + SqlDialect<SqlFunctionParameterNotation = PositionalNotationOnly>,
{
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<DB> QueryFragment<DB, PositionalOrNamedNotation> for FunctionParameter<false>
where
    DB: Backend + SqlDialect<SqlFunctionParameterNotation = PositionalOrNamedNotation>,
{
    fn walk_ast<'b>(&'b self, _out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<DB> QueryFragment<DB, PositionalOrNamedNotation> for FunctionParameter<true>
where
    DB: Backend + SqlDialect<SqlFunctionParameterNotation = PositionalOrNamedNotation>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql(self.name);
        out.push_sql("=>");
        Ok(())
    }
}
