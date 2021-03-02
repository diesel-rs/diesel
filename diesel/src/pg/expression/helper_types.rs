use crate::dsl::{AsExpr, AsExprOf, SqlTypeOf};
use crate::expression::grouped::Grouped;
use crate::sql_types::{Inet, VarChar};

/// The return type of `lhs.ilike(rhs)`
pub type ILike<Lhs, Rhs> = Grouped<super::operators::ILike<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of `lhs.not_ilike(rhs)`
pub type NotILike<Lhs, Rhs> = Grouped<super::operators::NotILike<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of `lhs.similar_to(rhs)`
pub type SimilarTo<Lhs, Rhs> = Grouped<super::operators::SimilarTo<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of `lhs.not_similar_to(rhs)`
pub type NotSimilarTo<Lhs, Rhs> =
    Grouped<super::operators::NotSimilarTo<Lhs, AsExprOf<Rhs, VarChar>>>;

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

/// The return type of `lsh.contains(rhs)`
pub type ContainsNet<Lhs, Rhs> = Grouped<super::operators::ContainsNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lsh.contains_or_eq(rhs)`
pub type ContainsNetLoose<Lhs, Rhs> =
    Grouped<super::operators::ContainsNetLoose<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lsh.is_contained_by(rhs)`
pub type IsContainedByNet<Lhs, Rhs> =
    Grouped<super::operators::IsContainedByNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lsh.is_contained_by_or_eq(rhs)`
pub type IsContainedByNetLoose<Lhs, Rhs> =
    Grouped<super::operators::IsContainedByNetLoose<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lhs.overlaps_with(rhs)`
pub type OverlapsWithNet<Lhs, Rhs> =
    Grouped<super::operators::OverlapsWith<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lsh.and(rhs)`
pub type AndNet<Lhs, Rhs> = Grouped<super::operators::AndNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lsh.or(rhs)`
pub type OrNet<Lhs, Rhs> = Grouped<super::operators::OrNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lsh.diff(rhs)`
pub type DifferenceNet<Lhs, Rhs> =
    Grouped<super::operators::DifferenceNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of `lsh.jsonb_merge(rhs)`
pub type JsonbMerge<Lhs, Rhs> =
    Grouped<super::operators::JsonbMerge<Lhs, AsExprOf<Rhs, SqlTypeOf<Lhs>>>>;
