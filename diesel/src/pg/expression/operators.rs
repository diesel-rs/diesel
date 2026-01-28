use crate::backend::{Backend, DieselReserveSpecialization};
use crate::expression::expression_types::NotSelectable;
use crate::expression::{Expression, TypedExpressionType, ValidGrouping};
use crate::pg::expression::expression_methods::{ArrayOrNullableArray, IntegerOrNullableInteger};
use crate::pg::Pg;
use crate::query_builder::update_statement::changeset::AssignmentTarget;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_dsl::positional_order_dsl::{IntoPositionalOrderExpr, PositionalOrderExpr};
use crate::sql_types::is_nullable::{
    IsOneNullable, IsSqlTypeNullable, MaybeNullable, NotNull, OneNullable,
};
use crate::sql_types::{
    Array, Bigint, Bool, DieselNumericOps, Inet, Integer, Jsonb, MaybeNullableType, OneIsNullable,
    SqlType, Text,
};
use crate::{Column, QueryResult};

__diesel_infix_operator!(IsDistinctFrom, " IS DISTINCT FROM ", ConstantNullability Bool, backend: Pg);
__diesel_infix_operator!(IsNotDistinctFrom, " IS NOT DISTINCT FROM ", ConstantNullability Bool, backend: Pg);
infix_operator!(OverlapsWith, " && ", backend: Pg);
infix_operator!(Contains, " @> ", backend: Pg);
infix_operator!(IsContainedBy, " <@ ", backend: Pg);
infix_operator!(ILike, " ILIKE ", backend: Pg);
infix_operator!(ExtendsRightTo, " &< ", backend: Pg);
infix_operator!(ExtendsLeftTo, " &> ", backend: Pg);
infix_operator!(NotILike, " NOT ILIKE ", backend: Pg);
infix_operator!(SimilarTo, " SIMILAR TO ", backend: Pg);
infix_operator!(NotSimilarTo, " NOT SIMILAR TO ", backend: Pg);
postfix_operator!(NullsFirst, " NULLS FIRST", NotSelectable, backend: Pg);
postfix_operator!(NullsLast, " NULLS LAST", NotSelectable, backend: Pg);
postfix_operator!(IsJson, " IS JSON", ConditionalNullability Bool, backend: Pg);
postfix_operator!(IsNotJson, " IS NOT JSON", ConditionalNullability Bool, backend: Pg);
postfix_operator!(IsJsonObject, " IS JSON OBJECT", ConditionalNullability Bool, backend: Pg);
postfix_operator!(IsNotJsonObject, " IS NOT JSON OBJECT", ConditionalNullability Bool, backend: Pg);
postfix_operator!(IsJsonArray, " IS JSON ARRAY", ConditionalNullability Bool, backend: Pg);
postfix_operator!(IsNotJsonArray, " IS NOT JSON ARRAY", ConditionalNullability Bool, backend: Pg);
postfix_operator!(IsJsonScalar, " IS JSON SCALAR", ConditionalNullability Bool, backend: Pg);
postfix_operator!(IsNotJsonScalar, " IS NOT JSON SCALAR", ConditionalNullability Bool, backend: Pg);
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
infix_operator!(RangeAdjacent, " -|- ", backend: Pg);
infix_operator!(RemoveFromJsonb, " - ", Jsonb, backend: Pg);
__diesel_infix_operator!(RetrieveAsObjectJson, " -> ", __diesel_internal_SameResultAsInput, backend: Pg);
__diesel_infix_operator!(
    RetrieveByPathAsObjectJson,
    " #> ",
    __diesel_internal_SameResultAsInput,
    backend: Pg
);
infix_operator!(RetrieveByPathAsTextJson, " #>> ", Text, backend: Pg);
infix_operator!(RemoveByPathFromJsonb, " #-", Jsonb, backend: Pg);

__diesel_infix_operator!(
    UnionsRange,
    " + ",
    __diesel_internal_SameResultAsInput,
    backend: Pg
);

__diesel_infix_operator!(
    DifferenceRange,
    " - ",
    __diesel_internal_SameResultAsInput,
    backend: Pg
);

__diesel_infix_operator!(
    IntersectionRange,
    " * ",
    __diesel_internal_SameResultAsInput,
    backend: Pg
);

impl<T: PositionalOrderExpr> PositionalOrderExpr for NullsFirst<T> {}
impl<T: PositionalOrderExpr> PositionalOrderExpr for NullsLast<T> {}

impl<T: PositionalOrderExpr> IntoPositionalOrderExpr for NullsFirst<T> {
    type Output = NullsFirst<T>;

    fn into_positional_expr(self) -> Self::Output {
        self
    }
}
impl<T: PositionalOrderExpr> IntoPositionalOrderExpr for NullsLast<T> {
    type Output = NullsLast<T>;

    fn into_positional_expr(self) -> Self::Output {
        self
    }
}

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

impl<L, R, ST> Expression for ArrayIndex<L, R>
where
    L: Expression<SqlType: SqlType + ArrayOrNullableArray<Inner = ST>>,
    R: Expression<SqlType: SqlType + IntegerOrNullableInteger>,
    ST: SqlType + TypedExpressionType,
    IsSqlTypeNullable<L::SqlType>:
        OneIsNullable<IsSqlTypeNullable<R::SqlType>, Out: MaybeNullableType<ST>>,
{
    type SqlType = MaybeNullable<IsOneNullable<L::SqlType, R::SqlType>, ST>;
}

impl_selectable_expression!(ArrayIndex<L, R>);

impl<L, R> QueryFragment<Pg> for ArrayIndex<L, R>
where
    L: QueryFragment<Pg>,
    R: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("(");
        self.array_expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        out.push_sql("[");
        self.index_expr.walk_ast(out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

// we cannot use the additional parenthesis for updates
#[derive(Debug)]
pub struct UpdateArrayIndex<L, R>(ArrayIndex<L, R>);

impl<L, R> QueryFragment<Pg> for UpdateArrayIndex<L, R>
where
    L: QueryFragment<Pg>,
    R: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.0.array_expr.walk_ast(out.reborrow())?;
        out.push_sql("[");
        self.0.index_expr.walk_ast(out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl<L, R> AssignmentTarget for ArrayIndex<L, R>
where
    L: Column,
    // Null value in array subscript is forbidden in assignment
    R: Expression<SqlType = Integer>,
{
    type Table = <L as Column>::Table;
    type QueryAstNode = UpdateArrayIndex<UncorrelatedColumn<L>, R>;

    fn into_target(self) -> Self::QueryAstNode {
        UpdateArrayIndex(ArrayIndex::new(
            UncorrelatedColumn(self.array_expr),
            self.index_expr,
        ))
    }
}

#[derive(Debug, Clone, Copy, QueryId, ValidGrouping)]
#[doc(hidden)]
pub struct ArraySlice<L, R1, R2> {
    pub(crate) array_expr: L,
    pub(crate) slice_start_expr: R1,
    pub(crate) slice_end_expr: R2,
}

impl<L, R1, R2> ArraySlice<L, R1, R2> {
    pub fn new(array_expr: L, slice_start_expr: R1, slice_end_expr: R2) -> Self {
        Self {
            array_expr,
            slice_start_expr,
            slice_end_expr,
        }
    }
}

impl<L, R1, R2, ST> Expression for ArraySlice<L, R1, R2>
where
    L: Expression<SqlType: SqlType + ArrayOrNullableArray<Inner = ST>>,
    R1: Expression<SqlType: SqlType + IntegerOrNullableInteger>,
    R2: Expression<SqlType: SqlType + IntegerOrNullableInteger>,
    ST: SqlType + TypedExpressionType,
    IsSqlTypeNullable<L::SqlType>: OneIsNullable<IsSqlTypeNullable<R1::SqlType>>,
    IsOneNullable<L::SqlType, R1::SqlType>: OneIsNullable<IsSqlTypeNullable<R2::SqlType>>,
    OneNullable<IsOneNullable<L::SqlType, R1::SqlType>, IsSqlTypeNullable<R2::SqlType>>:
        MaybeNullableType<Array<ST>>,
{
    type SqlType = MaybeNullable<
        OneNullable<IsOneNullable<L::SqlType, R1::SqlType>, IsSqlTypeNullable<R2::SqlType>>,
        Array<ST>,
    >;
}

impl_selectable_expression!(ArraySlice<L, R1, R2>);

impl<L, R1, R2> QueryFragment<Pg> for ArraySlice<L, R1, R2>
where
    L: QueryFragment<Pg>,
    R1: QueryFragment<Pg>,
    R2: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("(");
        self.array_expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        out.push_sql("[");
        self.slice_start_expr.walk_ast(out.reborrow())?;
        out.push_sql(":");
        self.slice_end_expr.walk_ast(out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

// we cannot use the additional parenthesis for updates
#[derive(Debug)]
pub struct UpdateArraySlice<L, R1, R2>(ArraySlice<L, R1, R2>);

impl<L, R1, R2> QueryFragment<Pg> for UpdateArraySlice<L, R1, R2>
where
    L: QueryFragment<Pg>,
    R1: QueryFragment<Pg>,
    R2: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.0.array_expr.walk_ast(out.reborrow())?;
        out.push_sql("[");
        self.0.slice_start_expr.walk_ast(out.reborrow())?;
        out.push_sql(":");
        self.0.slice_end_expr.walk_ast(out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl<L, R1, R2> AssignmentTarget for ArraySlice<L, R1, R2>
where
    L: Column,
    // Null value in array subscript is forbidden in assignment
    R1: Expression<SqlType = Integer>,
    R2: Expression<SqlType = Integer>,
{
    type Table = <L as Column>::Table;
    type QueryAstNode = UpdateArraySlice<UncorrelatedColumn<L>, R1, R2>;

    fn into_target(self) -> Self::QueryAstNode {
        UpdateArraySlice(ArraySlice::new(
            UncorrelatedColumn(self.array_expr),
            self.slice_start_expr,
            self.slice_end_expr,
        ))
    }
}

#[derive(Debug, Clone, Copy, QueryId, ValidGrouping)]
#[doc(hidden)]
pub struct ArraySliceFrom<L, R> {
    pub(crate) array_expr: L,
    pub(crate) slice_start_expr: R,
}

impl<L, R> ArraySliceFrom<L, R> {
    pub fn new(array_expr: L, slice_start_expr: R) -> Self {
        Self {
            array_expr,
            slice_start_expr,
        }
    }
}

impl<L, R, ST> Expression for ArraySliceFrom<L, R>
where
    L: Expression<SqlType: SqlType + ArrayOrNullableArray<Inner = ST>>,
    R: Expression<SqlType: SqlType + IntegerOrNullableInteger>,
    ST: SqlType + TypedExpressionType,
    IsSqlTypeNullable<L::SqlType>:
        OneIsNullable<IsSqlTypeNullable<R::SqlType>, Out: MaybeNullableType<Array<ST>>>,
{
    type SqlType = MaybeNullable<IsOneNullable<L::SqlType, R::SqlType>, Array<ST>>;
}

impl_selectable_expression!(ArraySliceFrom<L, R>);

impl<L, R> QueryFragment<Pg> for ArraySliceFrom<L, R>
where
    L: QueryFragment<Pg>,
    R: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("(");
        self.array_expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        out.push_sql("[");
        self.slice_start_expr.walk_ast(out.reborrow())?;
        out.push_sql(":]");
        Ok(())
    }
}

// we cannot use the additional parenthesis for updates
#[derive(Debug)]
pub struct UpdateArraySliceFrom<L, R>(ArraySliceFrom<L, R>);

impl<L, R> QueryFragment<Pg> for UpdateArraySliceFrom<L, R>
where
    L: QueryFragment<Pg>,
    R: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.0.array_expr.walk_ast(out.reborrow())?;
        out.push_sql("[");
        self.0.slice_start_expr.walk_ast(out.reborrow())?;
        out.push_sql(":]");
        Ok(())
    }
}

impl<L, R> AssignmentTarget for ArraySliceFrom<L, R>
where
    // Column cannot be null if slice boundaries are not fully specified
    L: Column<SqlType: SqlType<IsNull = NotNull>>,
    // Null value in array subscript is forbidden in assignment
    R: Expression<SqlType = Integer>,
{
    type Table = <L as Column>::Table;
    type QueryAstNode = UpdateArraySliceFrom<UncorrelatedColumn<L>, R>;

    fn into_target(self) -> Self::QueryAstNode {
        UpdateArraySliceFrom(ArraySliceFrom::new(
            UncorrelatedColumn(self.array_expr),
            self.slice_start_expr,
        ))
    }
}

#[derive(Debug, Clone, Copy, QueryId, ValidGrouping)]
#[doc(hidden)]
pub struct ArraySliceTo<L, R> {
    pub(crate) array_expr: L,
    pub(crate) slice_end_expr: R,
}

impl<L, R> ArraySliceTo<L, R> {
    pub fn new(array_expr: L, slice_end_expr: R) -> Self {
        Self {
            array_expr,
            slice_end_expr,
        }
    }
}

impl<L, R, ST> Expression for ArraySliceTo<L, R>
where
    L: Expression<SqlType: SqlType + ArrayOrNullableArray<Inner = ST>>,
    R: Expression<SqlType: SqlType + IntegerOrNullableInteger>,
    ST: SqlType + TypedExpressionType,
    IsSqlTypeNullable<L::SqlType>:
        OneIsNullable<IsSqlTypeNullable<R::SqlType>, Out: MaybeNullableType<Array<ST>>>,
{
    type SqlType = MaybeNullable<IsOneNullable<L::SqlType, R::SqlType>, Array<ST>>;
}

impl_selectable_expression!(ArraySliceTo<L, R>);

impl<L, R> QueryFragment<Pg> for ArraySliceTo<L, R>
where
    L: QueryFragment<Pg>,
    R: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("(");
        self.array_expr.walk_ast(out.reborrow())?;
        out.push_sql(")");
        out.push_sql("[:");
        self.slice_end_expr.walk_ast(out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

// we cannot use the additional parenthesis for updates
#[derive(Debug)]
pub struct UpdateArraySliceTo<L, R>(ArraySliceTo<L, R>);

impl<L, R> QueryFragment<Pg> for UpdateArraySliceTo<L, R>
where
    L: QueryFragment<Pg>,
    R: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        self.0.array_expr.walk_ast(out.reborrow())?;
        out.push_sql("[:");
        self.0.slice_end_expr.walk_ast(out.reborrow())?;
        out.push_sql("]");
        Ok(())
    }
}

impl<L, R> AssignmentTarget for ArraySliceTo<L, R>
where
    // Column cannot be null if slice boundaries are not fully specified
    L: Column<SqlType: SqlType<IsNull = NotNull>>,
    // Null value in array subscript is forbidden in assignment
    R: Expression<SqlType = Integer>,
{
    type Table = <L as Column>::Table;
    type QueryAstNode = UpdateArraySliceTo<UncorrelatedColumn<L>, R>;

    fn into_target(self) -> Self::QueryAstNode {
        UpdateArraySliceTo(ArraySliceTo::new(
            UncorrelatedColumn(self.array_expr),
            self.slice_end_expr,
        ))
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
