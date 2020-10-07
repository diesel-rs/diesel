use crate::dsl::{AsExpr, AsExprOf, SqlTypeOf};
use crate::expression::grouped::Grouped;
use crate::sql_types::VarChar;

/// The return type of `lhs.ilike(rhs)`
pub type ILike<Lhs, Rhs> = Grouped<super::operators::ILike<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of `lhs.not_ilike(rhs)`
pub type NotILike<Lhs, Rhs> = Grouped<super::operators::NotILike<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of `lhs.is_not_distinct_from(rhs)`
pub type IsNotDistinctFrom<Lhs, Rhs> =
    Grouped<super::operators::IsNotDistinctFrom<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.is_distinct_from(rhs)`
pub type IsDistinctFrom<Lhs, Rhs> =
    Grouped<super::operators::IsDistinctFrom<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.overlaps_with(rhs)`
pub type OverlapsWith<Lhs, Rhs> = Grouped<super::operators::OverlapsWith<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.contains(rhs)` for array expressions
pub type ArrayContains<Lhs, Rhs> = Grouped<super::operators::Contains<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `lhs.contains(rhs)` for range expressions
pub type RangeContains<Lhs, Rhs> = Grouped<
    super::operators::Contains<
        Lhs,
        AsExprOf<Rhs, <SqlTypeOf<Lhs> as super::expression_methods::RangeHelper>::Inner>,
    >,
>;

/// The return type of `lhs.is_contained_by(rhs)`
pub type IsContainedBy<Lhs, Rhs> = Grouped<super::operators::IsContainedBy<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of `expr.nulls_first()`
pub type NullsFirst<T> = super::operators::NullsFirst<T>;

/// The return type of `expr.nulls_last()`
pub type NullsLast<T> = super::operators::NullsLast<T>;

/// The return type of `expr.at_time_zone(tz)`
pub type AtTimeZone<Lhs, Rhs> =
    Grouped<super::date_and_time::AtTimeZone<Lhs, AsExprOf<Rhs, VarChar>>>;
