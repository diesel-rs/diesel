use crate::backend::{Backend, DieselReserveSpecialization};
use crate::expression::expression_types::NotSelectable;
use crate::expression::{TypedExpressionType, ValidGrouping};
use crate::pg::Pg;
use crate::query_builder::update_statement::changeset::AssignmentTarget;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::sql_types::{
    Array, Bigint, Bool, DieselNumericOps, Inet, Integer, Jsonb, SqlType, Text,
};
use crate::{Column, QueryResult};

__diesel_infix_operator!(IsDistinctFrom, " IS DISTINCT FROM ", ConstantNullability Bool, backend: Pg);
__diesel_infix_operator!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", ConstantNullability Bool, backend: Pg);
infix_operator!(OverlapsWith, " && ", backend: Pg);
infix_operator!(Contains, " @> ", backend: Pg);
infix_operator!(IsContainedBy, " <@ ", backend: Pg);
infix_operator!(ILike, " ILIKE ", backend: Pg);
infix_operator!(NotILike, " NOT ILIKE ", backend: Pg);
infix_operator!(SimilarTo, " SIMILAR TO ", backend: Pg);
infix_operator!(NotSimilarTo, " NOT SIMILAR TO ", backend: Pg);
postfix_operator!(NullsFirst, " NULLS FIRST", NotSelectable, backend: Pg);
postfix_operator!(NullsLast, " NULLS LAST", NotSelectable, backend: Pg);
infix_operator!(ContainsNet, " >> ", backend: Pg);
infix_operator!(ContainsNetLoose, " >>= ", backend: Pg);
infix_operator!(IsContainedByNet, " << ", backend: Pg);
infix_operator!(IsContainedByNetLoose, " <<= ", backend: Pg);
infix_operator!(AndNet, " & ", Inet, backend: Pg);
infix_operator!(OrNet, " | ", Inet, backend: Pg);
infix_operator!(DifferenceNet, " - ", Bigint, backend: Pg);
infix_operator!(HasKeyJsonb, " ? ", backend: Pg);
infix_operator!(HasAnyKeyJsonb, " ?| ", backend: Pg);
infix_operator!(HasAllKeysJsonb, " ?& ", backend: Pg);
infix_operator!(RemoveFromJsonb, " - ", Jsonb, backend: Pg);
__diesel_infix_operator!(
    RetrieveAsObjectJson,
    " -> ",
    __diesel_internal_SameResultAsInput,
    backend: Pg
);
infix_operator!(RetrieveAsTextJson, " ->> ", Text, backend: Pg);
__diesel_infix_operator!(
    RetrieveByPathAsObjectJson,
    " #> ",
    __diesel_internal_SameResultAsInput,
    backend: Pg
);
infix_operator!(RetrieveByPathAsTextJson, " #>> ", Text, backend: Pg);
infix_operator!(RemoveByPathFromJsonb, " #-", Jsonb, backend: Pg);

#[derive(Debug, Clone, Copy, QueryId, DieselNumericOps, ValidGrouping)]
#[doc(hidden)]
pub struct ArrayIndex<L, R> {
    pub(crate) array_expr: L,
    pub(crate) index_expr: R,
}

impl<L, R> ArrayIndex<L, R> {
    pub fn new(array_expr: L, index_expr: R) -> Self {
        Self {
            array_expr,
            index_expr,
        }
    }
}

impl<L, R, ST> crate::expression::Expression for ArrayIndex<L, R>
where
    L: crate::expression::Expression<SqlType = Array<ST>>,
    R: crate::expression::Expression<SqlType = Integer>,
    ST: SqlType + TypedExpressionType,
{
    type SqlType = ST;
}

impl_selectable_expression!(ArrayIndex<L, R>);

impl<L, R> QueryFragment<Pg> for ArrayIndex<L, R>
where
    L: QueryFragment<Pg>,
    R: QueryFragment<Pg>,
{
    fn walk_ast<'b>(
        &'b self,
        mut out: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::result::QueryResult<()> {
        self.array_expr.walk_ast(out.reborrow())?;
        out.push_sql("[");
        self.index_expr.walk_ast(out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl<L, R> AssignmentTarget for ArrayIndex<L, R>
where
    L: Column,
{
    type Table = <L as Column>::Table;
    type QueryAstNode = ArrayIndex<UncorrelatedColumn<L>, R>;

    fn into_target(self) -> Self::QueryAstNode {
        ArrayIndex::new(UncorrelatedColumn(self.array_expr), self.index_expr)
    }
}

/// Signifies that this column should be rendered without its 'correlation'
/// (i.e. table name prefix). For update statements, fully qualified column
/// names aren't allowed.
#[derive(Debug)]
pub struct UncorrelatedColumn<C>(C);

impl<C, DB> QueryFragment<DB> for UncorrelatedColumn<C>
where
    C: Column,
    DB: Backend + DieselReserveSpecialization,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_identifier(C::NAME)
    }
}
