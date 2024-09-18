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
/// and [`lhs.overlaps_with(rhs)`](super::expression_methods::PgRangeExpressionMethods::overlaps_with)
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

/// The return type of [`lhs.range_extends_right_to(rhs)`](super::expression_methods::PgRangeExpressionMethods::range_extends_right_to)
/// for range expressions
#[cfg(feature = "postgres_backend")]
pub type RangeExtendsRightTo<Lhs, Rhs> =
    Grouped<super::operators::ExtendsRightTo<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.range_extends_left_to(rhs)`](super::expression_methods::PgRangeExpressionMethods::range_extends_left_to)
/// for range expressions
#[cfg(feature = "postgres_backend")]
pub type RangeExtendsLeftTo<Lhs, Rhs> =
    Grouped<super::operators::ExtendsLeftTo<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.contains_range(rhs)`](super::expression_methods::PgRangeExpressionMethods::contains_range)
/// for range expressions
#[cfg(feature = "postgres_backend")]
pub type ContainsRange<Lhs, Rhs> = Contains<Lhs, Rhs>;

/// The return type of [`lhs.range_is_contained_by(rhs)`](super::expression_methods::PgRangeExpressionMethods::is_contained_by)
/// and [`lhs.is_contained_by(rhs)`](super::expression_methods::PgArrayExpressionMethods::is_contained_by)
#[cfg(feature = "postgres_backend")]
pub type IsContainedBy<Lhs, Rhs> = Grouped<super::operators::IsContainedBy<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.is_contained_by_range(rhs)`](super::expression_methods::PgExpressionMethods::is_contained_by_range)
#[cfg(feature = "postgres_backend")]
pub type IsContainedByRange<Lhs, Rhs> = Grouped<
    super::operators::IsContainedBy<Lhs, AsExprOf<Rhs, diesel::sql_types::Range<SqlTypeOf<Lhs>>>>,
>;

/// The return type of [`lhs.range_is_contained_by(rhs)`](super::expression_methods::PgRangeExpressionMethods::lesser_than)
#[cfg(feature = "postgres_backend")]
pub type LesserThanRange<Lhs, Rhs> =
    Grouped<super::operators::IsContainedByNet<Lhs, AsExpr<Rhs, Lhs>>>;

#[doc(hidden)] // used by `#[auto_type]`
pub type LesserThan<Lhs, Rhs> = LesserThanRange<Lhs, Rhs>;

/// The return type of [`lhs.range_is_contained_by(rhs)`](super::expression_methods::PgRangeExpressionMethods::greater_than)
#[cfg(feature = "postgres_backend")]
pub type GreaterThanRange<Lhs, Rhs> = Grouped<super::operators::ContainsNet<Lhs, AsExpr<Rhs, Lhs>>>;

#[doc(hidden)] // used by `#[auto_type]`
pub type GreaterThan<Lhs, Rhs> = GreaterThanRange<Lhs, Rhs>;

/// The return type of [`lhs.union_range(rhs)`](super::expression_methods::PgRangeExpressionMethods::union_range)
#[cfg(feature = "postgres_backend")]
pub type UnionRange<Lhs, Rhs> = Grouped<super::operators::UnionsRange<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.difference_range(rhs)`](super::expression_methods::PgRangeExpressionMethods::difference_range)
#[cfg(feature = "postgres_backend")]
pub type Difference<Lhs, Rhs> = Grouped<super::operators::DifferenceRange<Lhs, AsExpr<Rhs, Lhs>>>;

#[doc(hidden)] // used by `#[auto_type]`
pub type DifferenceRange<Lhs, Rhs> = Difference<Lhs, Rhs>;

/// The return type of [`lhs.range_adjacent(rhs)`](super::expression_methods::PgRangeExpressionMethods::range_adjacent)
#[cfg(feature = "postgres_backend")]
pub type RangeAdjacent<Lhs, Rhs> = Grouped<super::operators::RangeAdjacent<Lhs, AsExpr<Rhs, Lhs>>>;

/// The return type of [`lhs.intersection_range(rhs)`](super::expression_methods::PgRangeExpressionMethods::intersection_range)
#[cfg(feature = "postgres_backend")]
pub type Intersection<Lhs, Rhs> =
    Grouped<super::operators::IntersectionRange<Lhs, AsExpr<Rhs, Lhs>>>;

#[doc(hidden)] // used by `#[auto_type]`
pub type IntersectionRange<Lhs, Rhs> = Intersection<Lhs, Rhs>;

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

/// Return type of [`array_to_string_with_null_string(arr, delim, null_str)`](super::functions::array_to_string_with_null_string)
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_to_string_with_null_string<A, D, N> =
    super::functions::array_to_string_with_null_string<
        SqlTypeOf<A>, // The SQL type of the array
        A,            // The array itself
        D,            // The delimiter
        N,            // The null string
    >;

/// Return type of [`array_to_string(arr, delim)`](super::functions::array_to_string)
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_to_string<A, D> = super::functions::array_to_string<
    SqlTypeOf<A>, // The SQL type of the array
    A,            // The array itself
    D,            // The delimiter
>;

/// Return type of [`lower(range)`](super::functions::lower())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type lower<R> = super::functions::lower<SqlTypeOf<R>, R>;

/// Return type of [`upper(range)`](super::functions::upper())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type upper<R> = super::functions::upper<SqlTypeOf<R>, R>;

/// Return type of [`isempty(range)`](super::functions::isempty())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type isempty<R> = super::functions::isempty<SqlTypeOf<R>, R>;

/// Return type of [`lower_inc(range)`](super::functions::lower_inc())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type lower_inc<R> = super::functions::lower_inc<SqlTypeOf<R>, R>;

/// Return type of [`upper_inc(range)`](super::functions::upper_inc())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type upper_inc<R> = super::functions::upper_inc<SqlTypeOf<R>, R>;

/// Return type of [`lower_inf(range)`](super::functions::lower_inf())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type lower_inf<R> = super::functions::lower_inf<SqlTypeOf<R>, R>;

/// Return type of [`upper_inf(range)`](super::functions::upper_inf())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type upper_inf<R> = super::functions::upper_inf<SqlTypeOf<R>, R>;

/// Return type of [`range_merge(range_a, range_b)`](super::functions::range_merge())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type range_merge<R1, R2> = super::functions::range_merge<SqlTypeOf<R1>, SqlTypeOf<R2>, R1, R2>;

/// Return type of [`multirange_merge(multirange)`](super::functions::multirange_merge())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type multirange_merge<R> = super::functions::multirange_merge<SqlTypeOf<R>, R>;

/// Return type of [`array_append(array, element)`](super::functions::array_append())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_append<A, E> = super::functions::array_append<SqlTypeOf<A>, SqlTypeOf<E>, A, E>;

/// Return type of [`array_replace(array, element, replace_with)`](super::functions::array_replace())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_replace<A, E, R> =
    super::functions::array_replace<SqlTypeOf<A>, SqlTypeOf<E>, A, E, R>;

/// Return type of [`array_dims(array)`](super::functions::array_append())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_dims<A> = super::functions::array_dims<SqlTypeOf<A>, A>;

/// Return type of [`array_prepend(element, array)`](super::functions::array_prepend())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_prepend<E, A> = super::functions::array_prepend<SqlTypeOf<E>, SqlTypeOf<A>, E, A>;

/// Return type of [`array_remove(array, element)`](super::functions::array_remove())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_remove<A, E> = super::functions::array_remove<SqlTypeOf<A>, SqlTypeOf<E>, A, E>;

/// Return type of [`cardinality(array)`](super::functions::cardinality())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type cardinality<A> = super::functions::cardinality<SqlTypeOf<A>, A>;

/// Return type of [`trim_array(array)`](super::functions::trim_array())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type trim_array<A, N> = super::functions::trim_array<SqlTypeOf<A>, A, N>;

/// Return type of [`array_cat(array_a, array_b)`](super::functions::array_cat())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_cat<A, B> = super::functions::array_cat<SqlTypeOf<A>, A, B>;

/// Return type of [`array_length(array, dimension)`](super::functions::array_length())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_length<A, D> = super::functions::array_length<SqlTypeOf<A>, A, D>;

/// Return type of [`array_fill(value,array)`](super::functions::array_fill())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_fill<E, A> = super::functions::array_fill<SqlTypeOf<E>, E, A>;

/// Return type of [`array_fill_with_lower_bound(value,array,array)`](super::functions::array_fill_with_lower_bound())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_fill_with_lower_bound<E, A1, A2> =
    super::functions::array_fill_with_lower_bound<SqlTypeOf<E>, E, A1, A2>;

/// Return type of [`array_lower(array, bound)`](super::functions::array_lower())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_lower<A, D> = super::functions::array_lower<SqlTypeOf<A>, A, D>;

/// Return type of [`array_upper(array, bound)`](super::functions::array_upper())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_upper<A, D> = super::functions::array_upper<SqlTypeOf<A>, A, D>;

/// Return type of [`array_position(array,element)`](super::functions::array_position)
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_position<A, E> = super::functions::array_position<SqlTypeOf<A>, SqlTypeOf<E>, A, E>;

/// Return type of [`array_position_with_subscript(array,element,subscript)`](super::functions::array_position_with_subscript)
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_position_with_subscript<A, E, S> =
    super::functions::array_position_with_subscript<SqlTypeOf<A>, SqlTypeOf<E>, A, E, S>;

/// Return type of [`array_positions(array,element)`](super::functions::array_positions)
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_positions<A, E> =
    super::functions::array_positions<SqlTypeOf<A>, SqlTypeOf<E>, A, E>;

/// Return type of [`array_ndims(array)`](super::functions::array_ndims())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_ndims<A> = super::functions::array_ndims<SqlTypeOf<A>, A>;

/// Return type of [`array_shuffle(array)`](super::functions::array_shuffle())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_shuffle<A> = super::functions::array_shuffle<SqlTypeOf<A>, A>;

/// Return type of [`array_sample(array,n)`](super::function::array_sample())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type array_sample<A, N> = super::functions::array_sample<SqlTypeOf<A>, A, N>;

/// Return type of [`to_json(element)`](super::functions::to_json())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type to_json<E> = super::functions::to_json<SqlTypeOf<E>, E>;

/// Return type of [`to_jsonb(element)`](super::functions::to_jsonb())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type to_jsonb<E> = super::functions::to_jsonb<SqlTypeOf<E>, E>;

/// Return type of [`json_object(text_array)`](super::functions::json_object())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type json_object<A> = super::functions::json_object<SqlTypeOf<A>, A>;

/// Return type of [`json_object_with_keys_and_values(text_array, text_array)`](super::functions::json_object_with_keys_and_values())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type json_object_with_keys_and_values<K, V> =
    super::functions::json_object_with_keys_and_values<SqlTypeOf<K>, SqlTypeOf<V>, K, V>;

/// Return type of [`json_typeof(json)`](super::functions::json_typeof())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type json_typeof<E> = super::functions::json_typeof<SqlTypeOf<E>, E>;

/// Return type of [`jsonb_typeof(jsonb)`](super::functions::jsonb_typeof())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type jsonb_typeof<E> = super::functions::jsonb_typeof<SqlTypeOf<E>, E>;

/// Return type of [`jsonb_pretty(jsonb)`](super::functions::jsonb_pretty())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type jsonb_pretty<E> = super::functions::jsonb_pretty<SqlTypeOf<E>, E>;

/// Return type of [`json_strip_nulls(json)`](super::functions::json_strip_nulls())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type json_strip_nulls<E> = super::functions::json_strip_nulls<SqlTypeOf<E>, E>;

/// Return type of [`jsonb_strip_nulls(jsonb)`](super::functions::jsonb_strip_nulls())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type jsonb_strip_nulls<E> = super::functions::jsonb_strip_nulls<SqlTypeOf<E>, E>;

/// Return type of [`json_array_length(json)`](super::functions::json_array_length())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type json_array_length<E> = super::functions::json_array_length<SqlTypeOf<E>, E>;

/// Return type of [`jsonb_array_length(jsonb)`](super::functions::jsonb_array_length())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type jsonb_array_length<E> = super::functions::jsonb_array_length<SqlTypeOf<E>, E>;

/// Return type of [`jsonb_insert(target, path, value)`](super::functions::jsonb_insert())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type jsonb_insert<T, P, V> =
    super::functions::jsonb_insert<SqlTypeOf<T>, SqlTypeOf<P>, T, P, V>;

/// Return type of [`jsonb_insert(target, path, value, insert_after)`](super::functions::jsonb_insert_with_option_after())
#[allow(non_camel_case_types)]
#[cfg(feature = "postgres_backend")]
pub type jsonb_insert_with_option_after<T, P, V, I> =
    super::functions::jsonb_insert_with_option_after<SqlTypeOf<T>, SqlTypeOf<P>, T, P, V, I>;
