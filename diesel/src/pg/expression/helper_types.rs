use crate::dsl::{AsExpr, AsExprOf, SqlTypeOf};
use crate::expression::grouped::Grouped;
use crate::expression::Expression;
use crate::pg::expression::expression_methods::private::{JsonIndex, JsonRemoveIndex};
use crate::pg::types::sql_types::Array;
use crate::sql_types::{Inet, Integer, VarChar};

/// The return type of [`lhs.ilike(rhs)`](super::expression_methods::PgTextExpressionMethods::ilike)
#[cfg(feature = "postgres_backend")]
pub type ILike<Lhs, Rhs> = Grouped<super::operators::ILike<Lhs, AsExprOf<Rhs, VarChar>>>;
#[doc(hidden)] // required for #[auto_type]
pub type Ilike<Lhs, Rhs> = ILike<Lhs, Rhs>;

/// The return type of [`lhs.not_ilike(rhs)`](super::expression_methods::PgTextExpressionMethods::not_ilike)
#[cfg(feature = "postgres_backend")]
pub type NotILike<Lhs, Rhs> = Grouped<super::operators::NotILike<Lhs, AsExprOf<Rhs, VarChar>>>;
#[doc(hidden)] // required for #[auto_type]
pub type NotIlike<Lhs, Rhs> = NotILike<Lhs, Rhs>;

/// The return type of [`lhs.similar_to(rhs)`](super::expression_methods::PgTextExpressionMethods::similar_to)
#[cfg(feature = "postgres_backend")]
pub type SimilarTo<Lhs, Rhs> = Grouped<super::operators::SimilarTo<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of [`lhs.not_similar_to(rhs)`](super::expression_methods::PgTextExpressionMethods::not_similar_to)
#[cfg(feature = "postgres_backend")]
pub type NotSimilarTo<Lhs, Rhs> =
    Grouped<super::operators::NotSimilarTo<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of [`lhs.is_not_distinct_from(rhs)`](super::expression_methods::PgExpressionMethods::is_not_distinct_from)
#[cfg(feature = "postgres_backend")]
pub type IsNotDistinctFrom<Lhs, Rhs> =
    Grouped<super::operators::IsNotDistinctFrom<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.is_distinct_from(rhs)`](super::expression_methods::PgExpressionMethods::is_distinct_from)
#[cfg(feature = "postgres_backend")]
pub type IsDistinctFrom<Lhs, Rhs> =
    Grouped<super::operators::IsDistinctFrom<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.overlaps_with(rhs)`](super::expression_methods::PgArrayExpressionMethods::overlaps_with)
#[cfg(feature = "postgres_backend")]
pub type OverlapsWith<Lhs, Rhs> = Grouped<super::operators::OverlapsWith<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.contains(rhs)`](super::expression_methods::PgArrayExpressionMethods::contains)
/// for array expressions
#[cfg(feature = "postgres_backend")]
pub type Contains<Lhs, Rhs> = Grouped<super::operators::Contains<Lhs, AsExpr<Rhs, Lhs>>>;

#[doc(hidden)]
#[deprecated(note = "Use `Contains` instead")]
pub type ArrayContains<Lhs, Rhs> = Contains<Lhs, Rhs>;

/// The return type of [`lhs.contains(rhs)`](super::expression_methods::PgRangeExpressionMethods::contains)
/// for range expressions
#[cfg(feature = "postgres_backend")]
pub type RangeContains<Lhs, Rhs> = Grouped<
    super::operators::Contains<
        Lhs,
        AsExprOf<Rhs, <SqlTypeOf<Lhs> as super::expression_methods::RangeHelper>::Inner>,
    >,
>;

/// The return type of [`lhs.is_contained_by(rhs)`](super::expression_methods::PgArrayExpressionMethods::is_contained_by)
#[cfg(feature = "postgres_backend")]
pub type IsContainedBy<Lhs, Rhs> = Grouped<super::operators::IsContainedBy<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`expr.nulls_first()`](super::expression_methods::PgSortExpressionMethods::nulls_first)
#[cfg(feature = "postgres_backend")]
pub type NullsFirst<T> = super::operators::NullsFirst<T>;

/// The return type of [`expr.nulls_last()`](super::expression_methods::PgSortExpressionMethods::nulls_last)
#[cfg(feature = "postgres_backend")]
pub type NullsLast<T> = super::operators::NullsLast<T>;

/// The return type of [`expr.at_time_zone(tz)`](super::expression_methods::PgTimestampExpressionMethods::at_time_zone)
#[cfg(feature = "postgres_backend")]
pub type AtTimeZone<Lhs, Rhs> =
    Grouped<super::date_and_time::AtTimeZone<Lhs, AsExprOf<Rhs, VarChar>>>;

/// The return type of [`lhs.contains(rhs)`](super::expression_methods::PgNetExpressionMethods::contains)
/// for network types
#[cfg(feature = "postgres_backend")]
pub type ContainsNet<Lhs, Rhs> = Grouped<super::operators::ContainsNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of [`lhs.contains_or_eq(rhs)`](super::expression_methods::PgNetExpressionMethods::contains_or_eq)
#[cfg(feature = "postgres_backend")]
pub type ContainsNetLoose<Lhs, Rhs> =
    Grouped<super::operators::ContainsNetLoose<Lhs, AsExprOf<Rhs, Inet>>>;

#[doc(hidden)] // used by `#[auto_type]`
pub type ContainsOrEq<Lhs, Rhs> = ContainsNetLoose<Lhs, Rhs>;

/// The return type of [`lhs.is_contained_by(rhs)`]((super::expression_methods::PgNetExpressionMethods::is_contained_by)
/// for network types
#[cfg(feature = "postgres_backend")]
pub type IsContainedByNet<Lhs, Rhs> =
    Grouped<super::operators::IsContainedByNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of [`lhs.is_contained_by_or_eq(rhs)`](super::expression_methods::PgNetExpressionMethods::is_contained_by_or_eq)
#[cfg(feature = "postgres_backend")]
pub type IsContainedByNetLoose<Lhs, Rhs> =
    Grouped<super::operators::IsContainedByNetLoose<Lhs, AsExprOf<Rhs, Inet>>>;

#[doc(hidden)] // is used by `#[auto_type]`
pub type IsContainedByOrEq<Lhs, Rhs> = IsContainedByNetLoose<Lhs, Rhs>;

/// The return type of [`lhs.overlaps_with(rhs)`](super::expression_methods::PgNetExpressionMethods::overlaps_with)
/// for network types
#[cfg(feature = "postgres_backend")]
pub type OverlapsWithNet<Lhs, Rhs> =
    Grouped<super::operators::OverlapsWith<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of [`lsh.and(rhs)`](super::expression_methods::PgNetExpressionMethods::and) for network types
#[cfg(feature = "postgres_backend")]
pub type AndNet<Lhs, Rhs> = Grouped<super::operators::AndNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of [`lsh.or(rhs)`](super::expression_methods::PgNetExpressionMethods::or) for network types
#[cfg(feature = "postgres_backend")]
pub type OrNet<Lhs, Rhs> = Grouped<super::operators::OrNet<Lhs, AsExprOf<Rhs, Inet>>>;

/// The return type of [`lsh.diff(rhs)`](super::expression_methods::PgNetExpressionMethods::diff)
#[cfg(feature = "postgres_backend")]
pub type DifferenceNet<Lhs, Rhs> =
    Grouped<super::operators::DifferenceNet<Lhs, AsExprOf<Rhs, Inet>>>;

#[doc(hidden)] // used by `#[auto_type]`
pub type Diff<Lhs, Rhs> = DifferenceNet<Lhs, Rhs>;

#[doc(hidden)]
#[deprecated(note = "Use `dsl::Concat` instead")]
pub type ConcatJsonb<Lhs, Rhs> = crate::dsl::Concat<Lhs, Rhs>;

/// The return type of [`lsh.has_key(rhs)`](super::expression_methods::PgJsonbExpressionMethods::has_key)
#[cfg(feature = "postgres_backend")]
pub type HasKeyJsonb<Lhs, Rhs> =
    Grouped<super::operators::HasKeyJsonb<Lhs, AsExprOf<Rhs, VarChar>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type HasKey<Lhs, Rhs> = HasKeyJsonb<Lhs, Rhs>;

/// The return type of [`lsh.has_any_key(rhs)`](super::expression_methods::PgJsonbExpressionMethods::has_any_key)
#[cfg(feature = "postgres_backend")]
pub type HasAnyKeyJsonb<Lhs, Rhs> =
    Grouped<super::operators::HasAnyKeyJsonb<Lhs, AsExprOf<Rhs, Array<VarChar>>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type HasAnyKey<Lhs, Rhs> = HasAnyKeyJsonb<Lhs, Rhs>;

/// The return type of [`lsh.has_all_keys(rhs)`](super::expression_methods::PgJsonbExpressionMethods::has_all_keys)
#[cfg(feature = "postgres_backend")]
pub type HasAllKeysJsonb<Lhs, Rhs> =
    Grouped<super::operators::HasAllKeysJsonb<Lhs, AsExprOf<Rhs, Array<VarChar>>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type HasAllKeys<Lhs, Rhs> = HasAllKeysJsonb<Lhs, Rhs>;

#[doc(hidden)]
#[deprecated(note = "Use `dsl::Contains` instead")]
pub type ContainsJsonb<Lhs, Rhs> = Contains<Lhs, Rhs>;

#[doc(hidden)]
#[deprecated(note = "Use `dsl::IsContainedBy` instead")]
pub type IsContainedByJsonb<Lhs, Rhs> = IsContainedBy<Lhs, Rhs>;

/// The return type of [`lhs.index(rhs)`](super::expression_methods::PgArrayExpressionMethods::index)
#[cfg(feature = "postgres_backend")]
pub type Index<Lhs, Rhs> = super::operators::ArrayIndex<Lhs, AsExprOf<Rhs, Integer>>;

#[doc(hidden)]
#[deprecated(note = "Use `Index` instead")]
pub type ArrayIndex<Lhs, Rhs> = Index<Lhs, Rhs>;

/// The return type of [`lhs.remove(rhs)`](super::expression_methods::PgJsonbExpressionMethods::remove)
#[cfg(feature = "postgres_backend")]
pub type RemoveFromJsonb<Lhs, Rhs, ST> =
    Grouped<super::operators::RemoveFromJsonb<Lhs, AsExprOf<Rhs, ST>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type Remove<Lhs, Rhs> = RemoveFromJsonb<
    Lhs,
    <Rhs as JsonRemoveIndex>::Expression,
    <<Rhs as JsonRemoveIndex>::Expression as Expression>::SqlType,
>;

/// The return type of [`lhs.retrieve_as_object(rhs)`](super::expression_methods::PgAnyJsonExpressionMethods::retrieve_as_object)
#[cfg(feature = "postgres_backend")]
pub type RetrieveAsObjectJson<Lhs, Rhs, ST> =
    Grouped<super::operators::RetrieveAsObjectJson<Lhs, AsExprOf<Rhs, ST>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type RetrieveAsObject<Lhs, Rhs> = RetrieveAsObjectJson<
    Lhs,
    <Rhs as JsonIndex>::Expression,
    <<Rhs as JsonIndex>::Expression as Expression>::SqlType,
>;

/// The return type of [`lhs.retrieve_as_text(rhs)`](super::expression_methods::PgAnyJsonExpressionMethods::retrieve_as_text)
#[cfg(feature = "postgres_backend")]
pub type RetrieveAsTextJson<Lhs, Rhs, ST> =
    Grouped<super::operators::RetrieveAsTextJson<Lhs, AsExprOf<Rhs, ST>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type RetrieveAsText<Lhs, Rhs> = RetrieveAsTextJson<
    Lhs,
    <Rhs as JsonIndex>::Expression,
    <<Rhs as JsonIndex>::Expression as Expression>::SqlType,
>;

/// The return type of [`lhs.retrieve_by_path_as_object(rhs)`](super::expression_methods::PgAnyJsonExpressionMethods::retrieve_by_path_as_object)
#[cfg(feature = "postgres_backend")]
pub type RetrieveByPathAsObjectJson<Lhs, Rhs> =
    Grouped<super::operators::RetrieveByPathAsObjectJson<Lhs, AsExprOf<Rhs, Array<VarChar>>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type RetrieveByPathAsObject<Lhs, Rhs> = RetrieveByPathAsObjectJson<Lhs, Rhs>;

/// The return type of [`lhs.retrieve_by_path_as_text(rhs)`](super::expression_methods::PgAnyJsonExpressionMethods::retrieve_by_path_as_text)
#[cfg(feature = "postgres_backend")]
pub type RetrieveByPathAsTextJson<Lhs, Rhs> =
    Grouped<super::operators::RetrieveByPathAsTextJson<Lhs, AsExprOf<Rhs, Array<VarChar>>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type RetrieveByPathAsText<Lhs, Rhs> = RetrieveByPathAsTextJson<Lhs, Rhs>;

/// The return type of [`lhs.remove_by_path(rhs)`](super::expression_methods::PgJsonbExpressionMethods::remove_by_path)
#[cfg(feature = "postgres_backend")]
pub type RemoveByPathFromJsonb<Lhs, Rhs> =
    Grouped<super::operators::RemoveByPathFromJsonb<Lhs, AsExprOf<Rhs, Array<VarChar>>>>;

#[doc(hidden)] // needed for `#[auto_type]`
pub type RemoveByPath<Lhs, Rhs> = RemoveByPathFromJsonb<Lhs, Rhs>;

#[doc(hidden)]
#[deprecated(note = "Use `dsl::Concat` instead")]
pub type ConcatBinary<Lhs, Rhs> = crate::dsl::Concat<Lhs, Rhs>;

#[doc(hidden)]
#[deprecated(note = "Use `dsl::Like` instead")]
pub type LikeBinary<Lhs, Rhs> = crate::dsl::Like<Lhs, Rhs>;

#[doc(hidden)]
#[deprecated(note = "Use `dsl::NotLike` instead")]
pub type NotLikeBinary<Lhs, Rhs> = crate::dsl::NotLike<Lhs, Rhs>;

#[doc(hidden)]
#[deprecated(note = "Use `dsl::Concat` instead")]
pub type ConcatArray<Lhs, Rhs> = crate::dsl::Concat<Lhs, Rhs>;
