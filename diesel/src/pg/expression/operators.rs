use crate::expression::expression_types::NotSelectable;
use crate::expression::{TypedExpressionType, ValidGrouping};
use crate::pg::Pg;
use crate::query_builder::{QueryFragment, QueryId};
use crate::sql_types::{Array, Bigint, Bool, DieselNumericOps, Inet, Integer, Jsonb, SqlType};

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
infix_operator!(ConcatJsonb, " || ", Jsonb, backend: Pg);
infix_operator!(HasKeyJsonb, " ? ", backend: Pg);
infix_operator!(HasAnyKeyJsonb, " ?| ", backend: Pg);
infix_operator!(HasAllKeysJsonb, " ?& ", backend: Pg);
infix_operator!(ContainsJsonb, " @> ", backend: Pg);
infix_operator!(IsContainedByJsonb, " <@ ", backend: Pg);
infix_operator!(RemoveFromJsonb, " - ", Jsonb, backend: Pg);
__diesel_infix_operator!(
    RetrieveAsObjectJsonb,
    " -> ",
    __diesel_internal_SameResultAsInput,
    backend: Pg
);

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
