//! PostgreSQL specific expression methods

pub(in crate::pg) use self::private::{
    ArrayOrNullableArray, CombinedAllNullableValue, CombinedNullableValue, InetOrCidr, JsonIndex,
    JsonOrNullableJson, JsonOrNullableJsonOrJsonbOrNullableJsonb, JsonRemoveIndex,
    JsonbOrNullableJsonb, MaybeNullableValue, MultirangeOrNullableMultirange,
    MultirangeOrRangeMaybeNullable, RangeOrMultirange, RangeOrNullableRange,
    RecordOrNullableRecord, TextArrayOrNullableTextArray, TextOrNullableText,
};
use super::date_and_time::{AtTimeZone, DateTimeLike};
use super::operators::*;
use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::operators::{Asc, Concat, Desc, Like, NotLike};
use crate::expression::{AsExpression, Expression, IntoSql, TypedExpressionType};
use crate::pg::expression::expression_methods::private::BinaryOrNullableBinary;
use crate::sql_types::{Array, Inet, Integer, Range, SqlType, Text, VarChar};
use crate::EscapeExpressionMethods;

/// PostgreSQL specific methods which are present on all expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `IS NOT DISTINCT FROM` expression.
    ///
    /// This behaves identically to the `=` operator, except that `NULL` is
    /// treated as a normal value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let distinct = users.select(id).filter(name.is_distinct_from("Sean"));
    /// let not_distinct = users.select(id).filter(name.is_not_distinct_from("Sean"));
    /// assert_eq!(Ok(2), distinct.first(connection));
    /// assert_eq!(Ok(1), not_distinct.first(connection));
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_not_distinct_from<T>(self, other: T) -> dsl::IsNotDistinctFrom<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsNotDistinctFrom::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `IS DISTINCT FROM` expression.
    ///
    /// This behaves identically to the `!=` operator, except that `NULL` is
    /// treated as a normal value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let distinct = users.select(id).filter(name.is_distinct_from("Sean"));
    /// let not_distinct = users.select(id).filter(name.is_not_distinct_from("Sean"));
    /// assert_eq!(Ok(2), distinct.first(connection));
    /// assert_eq!(Ok(1), not_distinct.first(connection));
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_distinct_from<T>(self, other: T) -> dsl::IsDistinctFrom<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsDistinctFrom::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `<@` expression.
    ///
    /// This operator returns true whether a element is contained by a range
    ///
    /// This operator evaluates to true for the following cases:
    ///
    /// ```text
    /// self:     |
    /// other:  [-----]
    /// ```
    ///
    /// ```text
    /// self:  |
    /// other: [----]
    /// ```
    ///
    /// ```text
    /// self:       |
    /// other:  [----]
    /// ```
    ///
    /// This operator evaluates to false for the following cases:
    ///
    /// ```text
    /// self:           |
    /// other:  [-----]
    /// ```
    ///
    /// ```text
    /// self:  |
    /// other:   [----]
    /// ```
    ///
    /// ```text
    /// self:       |
    /// other: [----)
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Range<Integer>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::dsl::int4range;
    /// #     use diesel::sql_types::Integer;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///       let my_range = int4range(1, 5, diesel::sql_types::RangeBound::LowerBoundInclusiveUpperBoundExclusive);
    ///
    ///       let (first, second) = diesel::select((
    ///           4.into_sql::<Integer>().is_contained_by_range(my_range),
    ///           10.into_sql::<Integer>().is_contained_by_range(my_range)
    ///       )).get_result::<(bool, bool)>(conn)?;
    ///
    ///       assert_eq!(first, true);
    ///       assert_eq!(second, false);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_contained_by_range<T>(self, other: T) -> dsl::IsContainedByRange<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Range<Self::SqlType>>,
    {
        Grouped(IsContainedBy::new(self, other.as_expression()))
    }
}

impl<T: Expression> PgExpressionMethods for T {}

/// PostgreSQL specific methods present on timestamp expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgTimestampExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL "AT TIME ZONE" expression.
    ///
    /// When this is called on a `TIMESTAMP WITHOUT TIME ZONE` column,
    /// the value will be treated as if were in the given time zone,
    /// and then converted to UTC.
    ///
    /// When this is called on a `TIMESTAMP WITH TIME ZONE` column,
    /// the value will be converted to the given time zone,
    /// and then have its time zone information removed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     timestamps (timestamp) {
    /// #         timestamp -> Timestamp,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(all(feature = "postgres", feature = "chrono"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::timestamps::dsl::*;
    /// #     use chrono::*;
    /// #     let connection = &mut establish_connection();
    /// #     diesel::sql_query("CREATE TABLE timestamps (\"timestamp\"
    /// #         timestamp NOT NULL)").execute(connection)?;
    /// let christmas_morning = NaiveDate::from_ymd(2017, 12, 25)
    ///     .and_hms(8, 0, 0);
    /// diesel::insert_into(timestamps)
    ///     .values(timestamp.eq(christmas_morning))
    ///     .execute(connection)?;
    ///
    /// let utc_time = timestamps
    ///     .select(timestamp.at_time_zone("UTC"))
    ///     .first(connection)?;
    /// assert_eq!(christmas_morning, utc_time);
    ///
    /// let eastern_time = timestamps
    ///     .select(timestamp.at_time_zone("EST"))
    ///     .first(connection)?;
    /// let five_hours_later = christmas_morning + Duration::hours(5);
    /// assert_eq!(five_hours_later, eastern_time);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(all(feature = "postgres", feature = "chrono")))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn at_time_zone<T>(self, timezone: T) -> dsl::AtTimeZone<Self, T>
    where
        T: AsExpression<VarChar>,
    {
        Grouped(AtTimeZone::new(self, timezone.as_expression()))
    }
}

impl<T: Expression> PgTimestampExpressionMethods for T where T::SqlType: DateTimeLike {}

/// PostgreSQL specific methods present on array expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgArrayExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `&&` expression.
    ///
    /// This operator returns whether two arrays have common elements.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)")
    /// #           .execute(conn)
    /// #           .unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         tags.eq(vec!["cool", "awesome"]),
    ///         tags.eq(vec!["awesome", "great"]),
    ///         tags.eq(vec!["cool", "great"]),
    ///     ])
    ///     .execute(conn)?;
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.overlaps_with(vec!["horrid", "cool"]))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 3], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.overlaps_with(vec!["cool", "great"]))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2, 3], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.overlaps_with(vec!["horrid"]))
    ///     .load::<i32>(conn)?;
    /// assert!(data.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    fn overlaps_with<T>(self, other: T) -> dsl::OverlapsWith<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(OverlapsWith::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `@>` expression.
    ///
    /// This operator returns whether an array contains another array.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)")
    /// #         .execute(conn)
    /// #         .unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(tags.eq(vec!["cool", "awesome"]))
    ///     .execute(conn)?;
    ///
    /// let cool_posts = posts.select(id)
    ///     .filter(tags.contains(vec!["cool"]))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1], cool_posts);
    ///
    /// let amazing_posts = posts.select(id)
    ///     .filter(tags.contains(vec!["cool", "amazing"]))
    ///     .load::<i32>(conn)?;
    /// assert!(amazing_posts.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> dsl::Contains<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Contains::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `<@` expression.
    ///
    /// This operator returns whether an array is contained by another array.
    /// `foo.contains(bar)` is the same as `bar.is_contained_by(foo)`
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)")
    /// #         .execute(conn)
    /// #         .unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(tags.eq(vec!["cool", "awesome"]))
    ///     .execute(conn)?;
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.is_contained_by(vec!["cool", "awesome", "amazing"]))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.is_contained_by(vec!["cool"]))
    ///     .load::<i32>(conn)?;
    /// assert!(data.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_contained_by<T>(self, other: T) -> dsl::IsContainedBy<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsContainedBy::new(self, other.as_expression()))
    }
    /// Indexes a PostgreSQL array.
    ///
    /// This operator indexes in to an array to access a single element.
    ///
    /// Note that PostgreSQL arrays are 1-indexed, so `foo.index(1)` is the
    /// first element in the array.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)")
    /// #         .execute(conn)
    /// #         .unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         tags.eq(vec!["cool", "awesome"]),
    ///         tags.eq(vec!["splendid", "marvellous"]),
    ///    ])
    ///     .execute(conn)?;
    ///
    /// let data = posts.select(tags.index(id))
    ///     .load::<String>(conn)?;
    /// assert_eq!(vec!["cool", "marvellous"], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.index(1).eq("splendid"))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![2], data);
    /// #     Ok(())
    /// # }
    /// ```
    fn index<T>(self, other: T) -> dsl::Index<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Integer>,
    {
        ArrayIndex::new(self, other.as_expression())
    }

    /// Creates a PostgreSQL `||` expression.
    ///
    /// This operator concatenates two Array values and returns Array value
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         tags -> Array<VarChar>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)")
    /// #         .execute(conn)
    /// #         .unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(tags.eq(vec!["cool", "awesome"]))
    ///     .execute(conn)?;
    ///
    /// let res = posts.select(tags.concat(vec!["amazing"])).load::<Vec<String>>(conn)?;
    /// let expected_tags = vec!["cool", "awesome", "amazing"];
    /// assert_eq!(expected_tags, res[0]);
    /// #     Ok(())
    /// # }
    ///
    fn concat<T>(self, other: T) -> dsl::Concat<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Concat::new(self, other.as_expression()))
    }
}

impl<T> PgArrayExpressionMethods for T
where
    T: Expression,
    T::SqlType: ArrayOrNullableArray,
{
}

/// PostgreSQL expression methods related to sorting.
///
/// This trait is only implemented for `Asc` and `Desc`. Although `.asc` is
/// implicit if no order is given, you will need to call `.asc()` explicitly in
/// order to call these methods.
#[cfg(feature = "postgres_backend")]
pub trait PgSortExpressionMethods: Sized {
    /// Specify that nulls should come before other values in this ordering.
    ///
    /// Normally, nulls come last when sorting in ascending order and first
    /// when sorting in descending order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     nullable_numbers (nullable_number) {
    /// #         nullable_number -> Nullable<Integer>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::nullable_numbers::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE nullable_numbers (nullable_number INTEGER)").execute(connection)?;
    /// diesel::insert_into(nullable_numbers)
    ///     .values(&vec![
    ///         nullable_number.eq(None),
    ///         nullable_number.eq(Some(1)),
    ///         nullable_number.eq(Some(2)),
    ///     ])
    ///     .execute(connection)?;
    ///
    /// let asc_default_nulls = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.asc())
    ///     .load(connection)?;
    /// assert_eq!(vec![Some(1), Some(2), None], asc_default_nulls);
    ///
    /// let asc_nulls_first = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.asc().nulls_first())
    ///     .load(connection)?;
    /// assert_eq!(vec![None, Some(1), Some(2)], asc_nulls_first);
    /// #     Ok(())
    /// # }
    /// ```
    fn nulls_first(self) -> dsl::NullsFirst<Self> {
        NullsFirst::new(self)
    }

    /// Specify that nulls should come after other values in this ordering.
    ///
    /// Normally, nulls come last when sorting in ascending order and first
    /// when sorting in descending order.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     nullable_numbers (nullable_number) {
    /// #         nullable_number -> Nullable<Integer>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::nullable_numbers::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE nullable_numbers (nullable_number INTEGER)").execute(connection)?;
    /// diesel::insert_into(nullable_numbers)
    ///     .values(&vec![
    ///         nullable_number.eq(None),
    ///         nullable_number.eq(Some(1)),
    ///         nullable_number.eq(Some(2)),
    ///     ])
    ///     .execute(connection)?;
    ///
    /// let desc_default_nulls = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.desc())
    ///     .load(connection)?;
    /// assert_eq!(vec![None, Some(2), Some(1)], desc_default_nulls);
    ///
    /// let desc_nulls_last = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.desc().nulls_last())
    ///     .load(connection)?;
    /// assert_eq!(vec![Some(2), Some(1), None], desc_nulls_last);
    /// #     Ok(())
    /// # }
    /// ```
    fn nulls_last(self) -> dsl::NullsLast<Self> {
        NullsLast::new(self)
    }
}

impl<T> PgSortExpressionMethods for Asc<T> {}
impl<T> PgSortExpressionMethods for Desc<T> {}

/// PostgreSQL specific methods present on text expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgTextExpressionMethods: Expression + Sized {
    /// Creates a  PostgreSQL `ILIKE` expression
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let starts_with_s = animals
    ///     .select(species)
    ///     .filter(name.ilike("s%").or(species.ilike("s%")))
    ///     .get_results::<String>(connection)?;
    /// assert_eq!(vec!["spider"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn ilike<T>(self, other: T) -> dsl::ILike<Self, T>
    where
        T: AsExpression<Text>,
    {
        Grouped(ILike::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `NOT ILIKE` expression
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let doesnt_start_with_s = animals
    ///     .select(species)
    ///     .filter(name.not_ilike("s%").and(species.not_ilike("s%")))
    ///     .get_results::<String>(connection)?;
    /// assert_eq!(vec!["dog"], doesnt_start_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn not_ilike<T>(self, other: T) -> dsl::NotILike<Self, T>
    where
        T: AsExpression<Text>,
    {
        Grouped(NotILike::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `SIMILAR TO` expression
    ///
    /// # Example
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let starts_with_s = animals
    ///     .select(species)
    ///     .filter(name.similar_to("s%").or(species.similar_to("s%")))
    ///     .get_results::<String>(connection)?;
    /// assert_eq!(vec!["spider"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn similar_to<T>(self, other: T) -> dsl::SimilarTo<Self, T>
    where
        T: AsExpression<Text>,
    {
        Grouped(SimilarTo::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `NOT SIMILAR TO` expression
    ///
    /// # Example
    /// ```
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let doesnt_start_with_s = animals
    ///     .select(species)
    ///     .filter(name.not_similar_to("s%").and(species.not_similar_to("s%")))
    ///     .get_results::<String>(connection)?;
    /// assert_eq!(vec!["dog"], doesnt_start_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn not_similar_to<T>(self, other: T) -> dsl::NotSimilarTo<Self, T>
    where
        T: AsExpression<Text>,
    {
        Grouped(NotSimilarTo::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `IS JSON` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is a valid JSON
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::Text;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("1".into_sql::<Text>().is_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("(1,2,3)".into_sql::<Text>().is_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_json(self) -> dsl::IsJson<Self> {
        IsJson::new(self)
    }

    /// Creates a PostgreSQL `IS NOT JSON` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is not a valid JSON
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::Text;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("1".into_sql::<Text>().is_not_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_not_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_not_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("(1,2,3)".into_sql::<Text>().is_not_json())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_not_json(self) -> dsl::IsNotJson<Self> {
        IsNotJson::new(self)
    }

    /// Creates a PostgreSQL `IS JSON OBJECT` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is a valid JSON
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::Text;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("123".into_sql::<Text>().is_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("abc".into_sql::<Text>().is_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("\"abc\"".into_sql::<Text>().is_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_json_object(self) -> dsl::IsJsonObject<Self> {
        IsJsonObject::new(self)
    }

    /// Creates a PostgreSQL `IS NOT JSON OBJECT` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is not a valid JSON
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::Text;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("123".into_sql::<Text>().is_not_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("abc".into_sql::<Text>().is_not_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_not_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_not_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("\"abc\"".into_sql::<Text>().is_not_json_object())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_not_json_object(self) -> dsl::IsNotJsonObject<Self> {
        IsNotJsonObject::new(self)
    }

    /// Creates a PostgreSQL `IS JSON ARRAY` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is a valid JSON ARRAY
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::Text;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("123".into_sql::<Text>().is_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("abc".into_sql::<Text>().is_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("\"abc\"".into_sql::<Text>().is_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_json_array(self) -> dsl::IsJsonArray<Self> {
        IsJsonArray::new(self)
    }

    /// Creates a PostgreSQL `IS NOT JSON ARRAY` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is not a valid JSON ARRAY
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::Text;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("123".into_sql::<Text>().is_not_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("abc".into_sql::<Text>().is_not_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_not_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_not_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("\"abc\"".into_sql::<Text>().is_not_json_array())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_not_json_array(self) -> dsl::IsNotJsonArray<Self> {
        IsNotJsonArray::new(self)
    }

    /// Creates a PostgreSQL `IS JSON SCALAR` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is a valid JSON SCALAR
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Text, Nullable};
    /// #     use diesel::dsl::sql;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("123".into_sql::<Text>().is_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("abc".into_sql::<Text>().is_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("\"abc\"".into_sql::<Text>().is_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(sql::<Nullable<Text>>("NULL").is_json_scalar()).get_result::<Option<bool>>(conn)?;
    /// assert!(res.is_none());
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_json_scalar(self) -> dsl::IsJsonScalar<Self> {
        IsJsonScalar::new(self)
    }

    /// Creates a PostgreSQL `IS NOT JSON SCALAR` expression.
    /// Requires PostgreSQL>=16
    ///
    /// This operator returns true whether an object is not a valid JSON SCALAR
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::Text;
    /// #
    /// #     let conn = &mut establish_connection();
    /// #
    ///
    /// let res = diesel::select(("123".into_sql::<Text>().is_not_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// let res = diesel::select(("abc".into_sql::<Text>().is_not_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("{\"products\": [1,2,3]}".into_sql::<Text>().is_not_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("[1,2,3]".into_sql::<Text>().is_not_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, true);
    /// let res = diesel::select(("\"abc\"".into_sql::<Text>().is_not_json_scalar())).get_result::<bool>(conn)?;
    /// assert_eq!(res, false);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_not_json_scalar(self) -> dsl::IsNotJsonScalar<Self> {
        IsNotJsonScalar::new(self)
    }
}

impl<T> PgTextExpressionMethods for T
where
    T: Expression,
    T::SqlType: TextOrNullableText,
{
}

impl<T, U> EscapeExpressionMethods for Grouped<ILike<T, U>> {
    type TextExpression = ILike<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(crate::expression::operators::Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

impl<T, U> EscapeExpressionMethods for Grouped<NotILike<T, U>> {
    type TextExpression = NotILike<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(crate::expression::operators::Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

impl<T, U> EscapeExpressionMethods for Grouped<SimilarTo<T, U>> {
    type TextExpression = SimilarTo<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(crate::expression::operators::Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

impl<T, U> EscapeExpressionMethods for Grouped<NotSimilarTo<T, U>> {
    type TextExpression = NotSimilarTo<T, U>;

    fn escape(self, character: char) -> dsl::Escape<Self> {
        Grouped(crate::expression::operators::Escape::new(
            self.0,
            character.to_string().into_sql::<VarChar>(),
        ))
    }
}

/// PostgreSQL specific methods present on range expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgRangeExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `@>` expression.
    ///
    /// This operator returns true whether a range contains an specific element
    ///
    /// This operator evaluates to true for the following cases:
    ///
    /// ```text
    /// self:   [-----]
    /// other:    |
    /// ```
    ///
    /// ```text
    /// self:  [----]
    /// other: |
    /// ```
    ///
    /// ```text
    /// self:  [----]
    /// other:      |
    /// ```
    ///
    /// This operator evaluates to false for the following cases:
    ///
    /// ```text
    /// self:   [-----]
    /// other:          |
    /// ```
    ///
    /// ```text
    /// self:    [----]
    /// other: |
    /// ```
    ///
    /// ```text
    /// self:  [----)
    /// other:      |
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select((1..5).into_sql::<Range<Integer>>().contains(4)).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select((1..5).into_sql::<Range<Integer>>().contains(8)).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select((vec![1..5]).into_sql::<Multirange<Integer>>().contains(4)).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select((vec![1..5]).into_sql::<Multirange<Integer>>().contains(8)).first::<bool>(conn).unwrap());
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> dsl::RangeContains<Self, T>
    where
        Self::SqlType: RangeOrMultirange,
        <Self::SqlType as RangeOrMultirange>::Inner: SqlType + TypedExpressionType,
        T: AsExpression<<Self::SqlType as RangeOrMultirange>::Inner>,
    {
        Grouped(Contains::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `@>` expression.
    ///
    /// This operator returns true whether a range contains another range
    ///
    /// This operator evaluates to true for the following cases:
    ///
    /// ```text
    /// self:   [-------]
    /// other:     [--]
    /// ```
    ///
    /// ```text
    /// self:  [------]
    /// other: [------]
    /// ```
    ///
    /// This operator evaluates to false for the following cases:
    ///
    /// ```text
    /// self:   [-----]
    /// other:     [----]
    /// ```
    ///
    /// ```text
    /// self:    [----]
    /// other: [--------]
    /// ```
    ///
    /// ```text
    /// self:  [----)
    /// other: [----]
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select(
    ///     (1..5).into_sql::<Range<Integer>>().contains_range(1..5)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     (1..5).into_sql::<Range<Integer>>().contains_range(3..7)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..5].into_sql::<Multirange<Integer>>().contains_range(vec![1..5])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     vec![1..5].into_sql::<Multirange<Integer>>().contains_range(vec![3..7])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn contains_range<T>(self, other: T) -> dsl::ContainsRange<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Contains::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `<@` expression.
    ///
    /// This operator returns true whether a range is contained by another range
    ///
    /// This operator evaluates to true for the following cases:
    ///
    /// ```text
    /// self:   [-----]
    /// other: [-------]
    /// ```
    ///
    /// ```text
    /// self:  [----]
    /// other: [----]
    /// ```
    ///
    /// This operator evaluates to false for the following cases:
    ///
    /// ```text
    /// self:   [------]
    /// other:   [---]
    /// ```
    ///
    /// ```text
    /// self:   [----]
    /// other: [-----)
    /// ```
    ///
    /// ```text
    /// self:  [----]
    /// other:   [----]
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select(
    ///     (1..5).into_sql::<Range<Integer>>().is_contained_by(1..5)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     (1..5).into_sql::<Range<Integer>>().is_contained_by(3..7)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..5].into_sql::<Multirange<Integer>>().is_contained_by(vec![1..5])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     vec![1..5].into_sql::<Multirange<Integer>>().is_contained_by(vec![3..7])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_contained_by<T>(self, other: T) -> dsl::IsContainedBy<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsContainedBy::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `&&` expression.
    ///
    /// This operator returns true whether two ranges overlap.
    ///
    /// This operator evaluates to true for the following cases:
    ///
    /// ```text
    /// self:   [-----]
    /// other:    [-----]
    /// ```
    ///
    /// ```text
    /// self:     [----]
    /// other: [----]
    /// ```
    ///
    /// ```text
    /// self:     [----]
    /// other:  [-------]
    /// ```
    ///
    /// ```text
    /// self:   [----]
    /// other:  [----]
    /// ```
    ///
    /// ```text
    /// self:   [----]
    /// other:  [----)
    /// ```
    ///
    /// This operator evaluates to false for the following cases:
    ///
    /// ```text
    /// self:   [-----]
    /// other:          [-----]
    /// ```
    ///
    /// ```text
    /// self:       [----]
    /// other: [--]
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select(
    ///     (1..5).into_sql::<Range<Integer>>().overlaps_with(3..7)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     (1..5).into_sql::<Range<Integer>>().overlaps_with(10..15)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..5].into_sql::<Multirange<Integer>>().overlaps_with(vec![3..7])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     vec![1..5].into_sql::<Multirange<Integer>>().overlaps_with(vec![10..15])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn overlaps_with<T>(self, other: T) -> dsl::OverlapsWith<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(OverlapsWith::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `&<` expression.
    ///
    /// This operator returns true whether the argument range extend to the right of the current range
    ///
    /// Postgresql defines "extends" as does not have a lower bound smaller than the lower bound of the
    /// self range. That means the right hand side range can overlap parts of the left hand side
    /// range or be on the right side of the left hand side range
    ///
    /// The following constelations evaluate to true:
    /// ```text
    /// self:   [------)
    /// other:    [----)
    /// ```
    ///
    /// ```text
    /// self:  [----)
    /// other:         [----)
    /// ```
    ///
    /// ```text
    /// self:  [------)
    /// other:    [------)
    /// ```
    ///
    /// The following constelations evaluate to false:
    ///
    /// ```text
    /// self:             [------]
    /// other:    [----]
    /// ```
    ///
    /// ```text
    /// self:            [------]
    /// other:         [----]
    /// ```
    ///
    /// ```text
    /// self:  [------]
    /// other: [------)
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// #
    /// assert!(diesel::select(
    ///     (1..20).into_sql::<Range<Integer>>().range_extends_right_to(18..20)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(diesel::select(
    ///     (1..20).into_sql::<Range<Integer>>().range_extends_right_to(25..30)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     (1..20).into_sql::<Range<Integer>>().range_extends_right_to(-10..0)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..20].into_sql::<Multirange<Integer>>().range_extends_right_to(vec![18..20])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(diesel::select(
    ///     vec![1..20].into_sql::<Multirange<Integer>>().range_extends_right_to(vec![25..30])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     vec![1..20].into_sql::<Multirange<Integer>>().range_extends_right_to(vec![-10..0])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn range_extends_right_to<T>(self, other: T) -> dsl::RangeExtendsRightTo<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(ExtendsRightTo::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `&>` expression.
    ///
    /// This operator returns true whether a range does extend to the left of another
    ///
    /// Postgresql defines "extends" as does not have a upper bound greater than the upper bound of the
    /// self range. That means the right hand side range can overlap parts of the left hand side
    /// range or be on the left side of the left hand side range
    ///
    /// The following constelations evaluate to true:
    /// ```text
    /// self:        [------)
    /// other:    [----)
    /// ```
    ///
    /// ```text
    /// self:          [----)
    /// other: [----)
    /// ```
    ///
    /// ```text
    /// self:  [------)
    /// other:        [------)
    /// ```
    ///
    /// The following constelations evaluate to false:
    ///
    /// ```text
    /// self:   [--------]
    /// other:    [----]
    /// ```
    ///
    /// ```text
    /// self:  [------]
    /// other:          [----]
    /// ```
    ///
    /// ```text
    /// self:  [------]
    /// other: (------]
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// #
    /// assert!(diesel::select(
    ///     (1..20).into_sql::<Range<Integer>>().range_extends_left_to(-10..5)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(diesel::select(
    ///     (1..20).into_sql::<Range<Integer>>().range_extends_left_to(-10..-5)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     (1..20).into_sql::<Range<Integer>>().range_extends_left_to(25..30)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..20].into_sql::<Multirange<Integer>>().range_extends_left_to(vec![-10..5])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(diesel::select(
    ///     vec![1..20].into_sql::<Multirange<Integer>>().range_extends_left_to(vec![-10..-5])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     vec![1..20].into_sql::<Multirange<Integer>>().range_extends_left_to(vec![25..30])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn range_extends_left_to<T>(self, other: T) -> dsl::RangeExtendsLeftTo<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(ExtendsLeftTo::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `<<` expression.
    ///
    /// Is the first range strictly left of the second?
    ///
    /// The following constelations evaluate to true:
    /// ```text
    /// self:   [------)
    /// other:            [----)
    /// ```
    ///
    /// ```text
    /// self:  [----)
    /// other:      [----)
    /// ```
    ///
    /// The following constelations evaluate to false:
    ///
    /// ```text
    /// self:             [------]
    /// other:    [----]
    /// ```
    ///
    /// ```text
    /// self:     [------]
    /// other:         [----]
    /// ```
    ///
    /// ```text
    /// self:  [------]
    /// other:        [------)
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Range<Integer>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     use std::collections::Bound;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions INT4RANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         (versions.eq((Bound::Included(1), Bound::Included(2)))),
    ///         (versions.eq((Bound::Included(3), Bound::Included(4)))),
    ///         (versions.eq((Bound::Included(5), Bound::Included(6))))
    ///     ])
    ///     .execute(conn)?;
    ///
    /// let data = posts.select(id)
    ///     .filter(versions.lesser_than((Bound::Included(1), Bound::Included(4))))
    ///     .load::<i32>(conn)?;
    /// assert!(data.is_empty());
    ///
    /// let data = posts.select(id)
    ///     .filter(versions.lesser_than((Bound::Included(5), Bound::Included(8))))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], data);
    /// #     Ok(())
    /// # }
    /// ```
    fn lesser_than<T>(self, other: T) -> dsl::LesserThanRange<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsContainedByNet::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `>>` expression.
    ///
    /// Is the first range strictly right of the second?
    ///
    /// The following constelations evaluate to true:
    /// ```text
    /// self:          [------)
    /// other: [----)
    /// ```
    ///
    /// ```text
    /// self:        [----)
    /// other:  [----)
    /// ```
    ///
    /// The following constelations evaluate to false:
    ///
    /// ```text
    /// self:  [------]
    /// other:          [----]
    /// ```
    ///
    /// ```text
    /// self:     [------]
    /// other:         [----]
    /// ```
    ///
    /// ```text
    /// self:         [------]
    /// other: [------]
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Range<Integer>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     use std::collections::Bound;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions INT4RANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         (versions.eq((Bound::Included(1), Bound::Included(2)))),
    ///         (versions.eq((Bound::Included(3), Bound::Included(4)))),
    ///         (versions.eq((Bound::Included(5), Bound::Included(6))))
    ///     ])
    ///     .execute(conn)?;
    ///
    /// let data = posts.select(id)
    ///     .filter(versions.greater_than((Bound::Included(1), Bound::Included(2))))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![2, 3], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(versions.greater_than((Bound::Included(5), Bound::Included(8))))
    ///     .load::<i32>(conn)?;
    /// assert!(data.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    fn greater_than<T>(self, other: T) -> dsl::GreaterThanRange<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(ContainsNet::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL ` -|- ` expression.
    ///
    /// This operator evaluates to true if the two ranges are adjacent
    ///
    /// The following constelations evaluate to true:
    /// ```text
    /// self:   [------)
    /// other:         [----)
    /// ```
    ///
    /// ```text
    /// self:       [----)
    /// other: [----)
    /// ```
    ///
    /// ```text
    /// self:        [----)
    /// other: [----]
    /// ```
    ///
    /// ```text
    /// self:  [----]
    /// other:       [----]
    /// ```
    ///
    /// The following constelations evaluate to false:
    ///
    /// ```text
    /// self:        [------]
    /// other:    [----]
    /// ```
    ///
    /// ```text
    /// self:     [------]
    /// other:         [----]
    /// ```
    ///
    /// ```text
    /// self:  [------]
    /// other:        [------]
    /// ```
    ///
    /// ```text
    /// self:  [------]
    /// other:           [------]
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select(
    ///     (1..2).into_sql::<Range<Integer>>().range_adjacent(2..=6)
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     (4..7).into_sql::<Range<Integer>>().range_adjacent(2..=6)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..2].into_sql::<Multirange<Integer>>().range_adjacent(vec![2..=6])
    ///     ).first::<bool>(conn).unwrap());
    /// assert!(!diesel::select(
    ///     vec![4..7].into_sql::<Multirange<Integer>>().range_adjacent(vec![2..=6])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn range_adjacent<T>(self, other: T) -> dsl::RangeAdjacent<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(RangeAdjacent::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL ` + ` expression.
    ///
    /// This operator unions two ranges and returns the union.
    ///
    /// ```text
    /// self:   [------)
    /// other:      [----)
    /// result: [--------)
    /// ```
    ///
    /// ```text
    /// self:          [----)
    /// other: [----)
    /// result: error
    /// ```
    ///
    /// ```text
    /// self:      [----)
    /// other: [----]
    /// result [--------)
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select(
    ///     (1..=2).into_sql::<Range<Integer>>().union_range(2..=6).eq(1..=6)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..=2].into_sql::<Multirange<Integer>>().union_range(vec![1..=6]).eq(vec![1..=6])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn union_range<T>(self, other: T) -> dsl::UnionRange<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(UnionsRange::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL ` - ` expression.
    ///
    /// This operator takes two ranges and returns the difference.
    ///
    /// The second range must not be contained in the first in such a way that the
    /// difference would not be a single range.
    ///
    /// ```text
    /// self:   [------)
    /// other:      [----)
    /// result: [---)
    /// ```
    ///
    /// ```text
    /// self:      [----)
    /// other:  [----)
    /// result:      [--)
    /// ```
    ///
    /// ```text
    /// self:      [--------)
    /// other:       [----]
    /// result: error
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select(
    ///     (1..=8).into_sql::<Range<Integer>>().difference_range(3..=8).eq(1..3)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..=8].into_sql::<Multirange<Integer>>().difference_range(vec![3..=8]).eq(vec![1..3])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn difference_range<T>(self, other: T) -> dsl::Difference<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(DifferenceRange::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL ` * ` expression.
    ///
    /// This operator takes two ranges and returns the intersection.
    ///
    /// ```text
    /// self:   [------)
    /// other:      [----)
    /// result:     [--)
    /// ```
    ///
    /// ```text
    /// self:      [----)
    /// other:  [----)
    /// result:    [-)
    /// ```
    ///
    /// ```text
    /// self:    [--------)
    /// other:     [----]
    /// result:    [----]
    /// ```
    ///
    /// ```text
    /// self:    [--------)
    /// other:               [----]
    /// result: empty range
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use diesel::sql_types::{Integer, Range, Multirange};
    /// #     let conn = &mut establish_connection();
    /// assert!(diesel::select(
    ///     (1..=8).into_sql::<Range<Integer>>().intersection_range(3..=8).eq(3..=8)
    ///     ).first::<bool>(conn).unwrap());
    ///
    /// assert!(diesel::select(
    ///     vec![1..=8].into_sql::<Multirange<Integer>>().intersection_range(vec![3..=8]).eq(vec![3..=8])
    ///     ).first::<bool>(conn).unwrap());
    /// #     Ok(())
    /// # }
    /// ```
    fn intersection_range<T>(self, other: T) -> dsl::Intersection<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IntersectionRange::new(self, other.as_expression()))
    }
}

impl<T> PgRangeExpressionMethods for T
where
    T: Expression,
    T::SqlType: MultirangeOrRangeMaybeNullable,
{
}

/// PostgreSQL specific methods present between CIDR/INET expressions
#[cfg(feature = "postgres_backend")]
pub trait PgNetExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `>>` expression.
    ///
    /// This operator returns whether a subnet strictly contains another subnet or address.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
    ///                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.contains(IpNetwork::from_str("10.0.2.5").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.contains(IpNetwork::from_str("10.0.2.5/24").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![2], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.contains(IpNetwork::from_str("10.0.3.31").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![2], my_hosts);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> dsl::ContainsNet<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(ContainsNet::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `>>=` expression.
    ///
    /// This operator returns whether a subnet contains or is equal to another subnet.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
    ///                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.contains_or_eq(IpNetwork::from_str("10.0.2.5").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.contains_or_eq(IpNetwork::from_str("10.0.2.5/24").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.contains_or_eq(IpNetwork::from_str("10.0.3.31").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![2], my_hosts);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn contains_or_eq<T>(self, other: T) -> dsl::ContainsNetLoose<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(ContainsNetLoose::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `<<` expression.
    ///
    /// This operator returns whether a subnet or address is strictly contained by another subnet.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
    ///                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.is_contained_by(IpNetwork::from_str("10.0.2.5/24").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(my_hosts.len(), 0);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.is_contained_by(IpNetwork::from_str("10.0.3.31/23").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.is_contained_by(IpNetwork::from_str("10.0.3.31/22").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], my_hosts);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_contained_by<T>(self, other: T) -> dsl::IsContainedByNet<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(IsContainedByNet::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `>>=` expression.
    ///
    /// This operator returns whether a subnet is contained by or equal to another subnet.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
    ///                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.is_contained_by_or_eq(IpNetwork::from_str("10.0.2.5/24").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.is_contained_by_or_eq(IpNetwork::from_str("10.0.3.31/23").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], my_hosts);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_contained_by_or_eq<T>(self, other: T) -> dsl::IsContainedByNetLoose<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(IsContainedByNetLoose::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `&&` expression.
    ///
    /// This operator returns whether a subnet contains or is contained by another subnet.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
    ///                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.overlaps_with(IpNetwork::from_str("10.0.2.5/24").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.overlaps_with(IpNetwork::from_str("10.0.3.31/24").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![2], my_hosts);
    ///
    /// let my_hosts = hosts.select(id)
    ///     .filter(address.overlaps_with(IpNetwork::from_str("10.0.3.31/23").unwrap()))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1, 2], my_hosts);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn overlaps_with<T>(self, other: T) -> dsl::OverlapsWithNet<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(OverlapsWith::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `&` expression.
    ///
    /// This operator computes the bitwise AND between two network addresses.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let addr = hosts
    ///     .select(address.and(IpNetwork::from_str("0.0.0.255").unwrap()))
    ///     .first::<IpNetwork>(conn)?;
    /// assert_eq!(addr, IpNetwork::from_str("0.0.0.3").unwrap());
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn and<T>(self, other: T) -> dsl::AndNet<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(AndNet::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `|` expression.
    ///
    /// This operator computes the bitwise OR between two network addresses.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let addr = hosts
    ///     .select(address.or(IpNetwork::from_str("0.0.0.255").unwrap()))
    ///     .first::<IpNetwork>(conn)?;
    /// assert_eq!(addr, IpNetwork::from_str("10.0.2.255").unwrap());
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn or<T>(self, other: T) -> dsl::OrNet<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(OrNet::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `-` expression.
    ///
    /// This operator subtracts an address from an address to compute the distance between the two
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     hosts {
    /// #         id -> Integer,
    /// #         address -> Inet,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "ipnetwork")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::hosts::dsl::*;
    /// #     use ipnetwork::IpNetwork;
    /// #     use std::str::FromStr;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS hosts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").execute(conn).unwrap();
    /// diesel::insert_into(hosts)
    ///     .values(vec![address.eq(IpNetwork::from_str("10.0.2.53").unwrap())])
    ///     .execute(conn)?;
    ///
    /// let offset = hosts
    ///     .select(address.diff(IpNetwork::from_str("10.0.2.42").unwrap()))
    ///     .first::<i64>(conn)?;
    /// assert_eq!(offset, 11);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "ipnetwork"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn diff<T>(self, other: T) -> dsl::DifferenceNet<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(DifferenceNet::new(self, other.as_expression()))
    }
}

impl<T> PgNetExpressionMethods for T
where
    T: Expression,
    T::SqlType: InetOrCidr,
{
}

/// PostgreSQL specific methods present on JSONB expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgJsonbExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `||` expression.
    ///
    /// This operator concatenates two JSONB values and returns JSONB value
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let to_concatenate: serde_json::Value = serde_json::json!({
    ///     "continent": "NA",
    ///     "planet": "Earth"
    /// });
    ///
    /// let final_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska",
    ///     "continent": "NA",
    ///     "planet": "Earth"
    /// });
    ///
    /// let final_address_db = contacts.select(address.concat(&to_concatenate)).get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(final_address, final_address_db);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn concat<T>(self, other: T) -> dsl::Concat<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Concat::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `?` expression.
    ///
    /// This operator checks if the right hand side string exists as a top-level key within the JSONB
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let key_exists = contacts.select(address.has_key("street")).get_result::<bool>(conn)?;
    /// assert!(key_exists);
    ///
    /// let santas_with_address_postcode = contacts.select(id).filter(address.has_key("postcode")).get_result::<i32>(conn)?;
    /// assert_eq!(1, santas_with_address_postcode);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn has_key<T>(self, other: T) -> dsl::HasKeyJsonb<Self, T>
    where
        T: AsExpression<VarChar>,
    {
        Grouped(HasKeyJsonb::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `?|` expression.
    ///
    /// This operator checks if any of the strings in the right hand side array exists as top level key in the given JSONB
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let any_key_exists = contacts.select(address.has_any_key(vec!["street", "city", "rudolf"])).get_result::<bool>(conn)?;
    /// assert!(any_key_exists);
    ///
    /// let santas_with_address_postcode = contacts.select(id).filter(address.has_any_key(vec!["street", "city", "rudolf"])).get_result::<i32>(conn)?;
    /// assert_eq!(1, santas_with_address_postcode);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ``````
    fn has_any_key<T>(self, other: T) -> dsl::HasAnyKeyJsonb<Self, T>
    where
        T: AsExpression<Array<VarChar>>,
    {
        Grouped(HasAnyKeyJsonb::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `?&` expression.
    ///
    /// This operator checks if all the strings in the right hand side array exist as top level keys in the given JSONB
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let all_keys_exist = contacts.select(address.has_all_keys(vec!["street", "city", "postcode"])).get_result::<bool>(conn)?;
    /// assert!(all_keys_exist);
    ///
    /// let santas_with_address_postcode = contacts.select(id).filter(address.has_all_keys(vec!["street", "city", "postcode"])).get_result::<i32>(conn)?;
    /// assert_eq!(1, santas_with_address_postcode);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ``````
    fn has_all_keys<T>(self, other: T) -> dsl::HasAllKeysJsonb<Self, T>
    where
        T: AsExpression<Array<VarChar>>,
    {
        Grouped(HasAllKeysJsonb::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `@>` expression.
    ///
    /// This operator checks whether left hand side JSONB value contains right hand side JSONB value
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let easter_bunny_address: serde_json::Value = serde_json::json!({
    ///     "street": "123 Carrot Road",
    ///     "province": "Easter Island",
    ///     "region": "Valparaíso",
    ///     "country": "Chile",
    ///     "postcode": "88888",
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Bunny"), address.eq(&easter_bunny_address)))
    ///     .execute(conn)?;
    ///
    /// let country_chile: serde_json::Value = serde_json::json!({"country": "Chile"});
    /// let contains_country_chile = contacts.select(address.contains(&country_chile)).get_result::<bool>(conn)?;
    /// assert!(contains_country_chile);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> dsl::Contains<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Contains::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `<@` expression.
    ///
    /// This operator checks whether left hand side JSONB value is contained by right hand side JSON value.
    /// `foo.contains(bar)` is the same as `bar.is_contained_by(foo)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let partial_easter_bunny_address: serde_json::Value = serde_json::json!({
    ///     "street": "123 Carrot Road",
    ///     "country": "Chile",
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Bunny"), address.eq(&partial_easter_bunny_address)))
    ///     .execute(conn)?;
    ///
    /// let full_easter_bunny_address: serde_json::Value = serde_json::json!({
    ///     "street": "123 Carrot Road",
    ///     "province": "Easter Island",
    ///     "region": "Valparaíso",
    ///     "country": "Chile",
    ///     "postcode": "88888",
    /// });
    /// let address_is_contained_by = contacts.select(address.is_contained_by(&full_easter_bunny_address)).get_result::<bool>(conn)?;
    /// assert!(address_is_contained_by);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_contained_by<T>(self, other: T) -> dsl::IsContainedBy<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsContainedBy::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `-` expression.
    ///
    /// This operator removes the value associated with the given key, that is provided on the
    /// Right Hand Side of the operator.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn)
    /// #        .unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let santas_modified_address = contacts.select(address.remove("postcode")).get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(santas_modified_address, serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "state": "Alaska"
    /// }));
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let santas_modified_address = contacts.select(address.remove(vec!["postcode", "state"])).get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(santas_modified_address, serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    /// }));
    ///
    /// let robert_downey_jr_addresses: serde_json::Value = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    ///     }
    /// ]);
    ///
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Robert Downey Jr."), address.eq(&robert_downey_jr_addresses)))
    ///     .execute(conn)?;
    ///
    /// let roberts_address_in_db = contacts
    ///                             .filter(name.eq("Robert Downey Jr."))
    ///                             .select(address.remove(1))
    ///                             .get_result::<serde_json::Value>(conn)?;
    ///
    /// let roberts_first_address = serde_json::json!([{
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    /// }]);
    /// assert_eq!(roberts_first_address, roberts_address_in_db);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    ///
    fn remove<T>(
        self,
        other: T,
    ) -> dsl::RemoveFromJsonb<Self, T::Expression, <T::Expression as Expression>::SqlType>
    where
        T: JsonRemoveIndex,
        <T::Expression as Expression>::SqlType: SqlType,
    {
        Grouped(RemoveFromJsonb::new(
            self,
            other.into_json_index_expression(),
        ))
    }

    /// Creates a PostgreSQL `#-` expression.
    ///
    /// This operator removes the value associated with the given json path, that is provided on the
    /// Right Hand Side of the operator.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn)
    /// #        .unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let santas_modified_address = contacts.select(address.remove("postcode")).get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(santas_modified_address, serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "state": "Alaska"
    /// }));
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let santas_modified_address = contacts.select(address.remove_by_path(vec!["postcode"])).get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(santas_modified_address, serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "state": "Alaska"
    /// }));
    ///
    /// let robert_downey_jr_addresses: serde_json::Value = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    ///     }
    /// ]);
    ///
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Robert Downey Jr."), address.eq(&robert_downey_jr_addresses)))
    ///     .execute(conn)?;
    ///
    /// let roberts_address_in_db = contacts
    ///                             .filter(name.eq("Robert Downey Jr."))
    ///                             .select(address.remove_by_path(vec!["1", "postcode"]))
    ///                             .get_result::<serde_json::Value>(conn)?;
    ///
    /// let roberts_address = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "state": "New York"
    ///     }
    /// ]);
    /// assert_eq!(roberts_address, roberts_address_in_db);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    ///
    fn remove_by_path<T>(self, other: T) -> dsl::RemoveByPathFromJsonb<Self, T::Expression>
    where
        T: AsExpression<Array<Text>>,
    {
        Grouped(RemoveByPathFromJsonb::new(self, other.as_expression()))
    }
}

impl<T> PgJsonbExpressionMethods for T
where
    T: Expression,
    T::SqlType: JsonbOrNullableJsonb,
{
}

/// PostgreSQL specific methods present on JSON and JSONB expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgAnyJsonExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `->` expression.
    ///
    /// This operator extracts the value associated with the given key, that is provided on the
    /// Right Hand Side of the operator.
    ///
    /// Extracts n'th element of JSON array (array elements are indexed from zero, but negative integers count from the end).
    /// Extracts JSON object field with the given key.
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn)
    /// #        .unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let santas_postcode = contacts.select(address.retrieve_as_object("postcode")).get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(santas_postcode, serde_json::json!("99705"));
    ///
    ///
    /// let robert_downey_jr_addresses: serde_json::Value = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    ///     }
    /// ]);
    ///
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Robert Downey Jr."), address.eq(&robert_downey_jr_addresses)))
    ///     .execute(conn)?;
    ///
    /// let roberts_second_address_in_db = contacts
    ///                             .filter(name.eq("Robert Downey Jr."))
    ///                             .select(address.retrieve_as_object(1))
    ///                             .get_result::<serde_json::Value>(conn)?;
    ///
    /// let roberts_second_address = serde_json::json!({
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    /// });
    /// assert_eq!(roberts_second_address, roberts_second_address_in_db);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn retrieve_as_object<T>(
        self,
        other: T,
    ) -> dsl::RetrieveAsObjectJson<Self, T::Expression, <T::Expression as Expression>::SqlType>
    where
        T: JsonIndex,
        <T::Expression as Expression>::SqlType: SqlType,
    {
        Grouped(RetrieveAsObjectJson::new(
            self,
            other.into_json_index_expression(),
        ))
    }

    /// Creates a PostgreSQL `->>` expression.
    ///
    /// This operator extracts the value associated with the given key, that is provided on the
    /// Right Hand Side of the operator.
    ///
    /// Extracts n'th element of JSON array (array elements are indexed from zero, but negative integers count from the end).
    /// Extracts JSON object field as Text with the given key.
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn)
    /// #        .unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let santas_postcode = contacts.select(address.retrieve_as_text("postcode")).get_result::<String>(conn)?;
    /// assert_eq!(santas_postcode, "99705");
    ///
    ///
    /// let robert_downey_jr_addresses: serde_json::Value = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    ///     }
    /// ]);
    ///
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Robert Downey Jr."), address.eq(&robert_downey_jr_addresses)))
    ///     .execute(conn)?;
    ///
    /// let roberts_second_address_in_db = contacts
    ///                             .filter(name.eq("Robert Downey Jr."))
    ///                             .select(address.retrieve_as_text(1))
    ///                             .get_result::<String>(conn)?;
    ///
    /// let roberts_second_address = String::from(
    ///     "{\"city\": \"New York\", \
    ///     \"state\": \"New York\", \
    ///     \"street\": \"Somewhere In Ny 251\", \
    ///     \"postcode\": \"3213212\"}"
    ///     );
    /// assert_eq!(roberts_second_address, roberts_second_address_in_db);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn retrieve_as_text<T>(
        self,
        other: T,
    ) -> dsl::RetrieveAsTextJson<Self, T::Expression, <T::Expression as Expression>::SqlType>
    where
        T: JsonIndex,
        <T::Expression as Expression>::SqlType: SqlType,
    {
        Grouped(RetrieveAsTextJson::new(
            self,
            other.into_json_index_expression(),
        ))
    }

    /// Creates a PostgreSQL `#>` expression.
    ///
    /// This operator extracts the value associated with the given key, that is provided on the
    /// Right Hand Side of the operator.
    ///
    /// Extracts n'th element of JSON array (array elements are indexed from zero, but negative integers count from the end).
    /// Extracts JSON object field with the given key.
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn)
    /// #        .unwrap();
    /// #
    /// let robert_downey_jr_addresses: serde_json::Value = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    ///     }
    /// ]);
    ///
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Robert Downey Jr."), address.eq(&robert_downey_jr_addresses)))
    ///     .execute(conn)?;
    ///
    /// let roberts_second_street_in_db = contacts
    ///                             .filter(name.eq("Robert Downey Jr."))
    ///                             .select(address.retrieve_by_path_as_object(vec!["1", "street"]))
    ///                             .get_result::<serde_json::Value>(conn)?;
    ///
    /// assert_eq!(roberts_second_street_in_db, serde_json::json!("Somewhere In Ny 251"));
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn retrieve_by_path_as_object<T>(
        self,
        other: T,
    ) -> dsl::RetrieveByPathAsObjectJson<Self, T::Expression>
    where
        T: AsExpression<Array<Text>>,
    {
        Grouped(RetrieveByPathAsObjectJson::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `#>>` expression.
    ///
    /// This operator extracts the value associated with the given key, that is provided on the
    /// Right Hand Side of the operator.
    ///
    /// Extracts n'th element of JSON array (array elements are indexed from zero, but negative integers count from the end).
    /// Extracts JSON object field as Text with the given key.
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn)
    /// #        .unwrap();
    /// #
    /// let robert_downey_jr_addresses: serde_json::Value = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    ///     }
    /// ]);
    ///
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Robert Downey Jr."), address.eq(&robert_downey_jr_addresses)))
    ///     .execute(conn)?;
    ///
    /// let roberts_second_street_in_db = contacts
    ///                             .filter(name.eq("Robert Downey Jr."))
    ///                             .select(address.retrieve_by_path_as_text(vec!["1", "street"]))
    ///                             .get_result::<String>(conn)?;
    ///
    /// assert_eq!(roberts_second_street_in_db, "Somewhere In Ny 251");
    ///
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn retrieve_by_path_as_text<T>(
        self,
        other: T,
    ) -> dsl::RetrieveByPathAsTextJson<Self, T::Expression>
    where
        T: AsExpression<Array<Text>>,
    {
        Grouped(RetrieveByPathAsTextJson::new(self, other.as_expression()))
    }
}

#[doc(hidden)]
impl<T> PgAnyJsonExpressionMethods for T
where
    T: Expression,
    T::SqlType: JsonOrNullableJsonOrJsonbOrNullableJsonb,
{
}

/// PostgreSQL specific methods present on Binary expressions.
#[cfg(feature = "postgres_backend")]
pub trait PgBinaryExpressionMethods: Expression + Sized {
    /// Concatenates two PostgreSQL byte arrays using the `||` operator.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> Binary,
    /// #         hair_color -> Nullable<Binary>,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::insert_into;
    /// #
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE users (
    /// #         id INTEGER PRIMARY KEY,
    /// #         name BYTEA NOT NULL,
    /// #         hair_color BYTEA
    /// #     )").execute(connection).unwrap();
    /// #
    /// #     insert_into(users)
    /// #         .values(&vec![
    /// #             (id.eq(1), name.eq("Sean".as_bytes()), Some(hair_color.eq(Some("Green".as_bytes())))),
    /// #             (id.eq(2), name.eq("Tess".as_bytes()), None),
    /// #         ])
    /// #         .execute(connection)
    /// #         .unwrap();
    /// #
    /// let names = users.select(name.concat(" the Greatest".as_bytes())).load(connection);
    /// let expected_names = vec![
    ///     b"Sean the Greatest".to_vec(),
    ///     b"Tess the Greatest".to_vec()
    /// ];
    /// assert_eq!(Ok(expected_names), names);
    ///
    /// // If the value is nullable, the output will be nullable
    /// let names = users.select(hair_color.concat("ish".as_bytes())).load(connection);
    /// let expected_names = vec![
    ///     Some(b"Greenish".to_vec()),
    ///     None,
    /// ];
    /// assert_eq!(Ok(expected_names), names);
    /// # }
    /// ```
    fn concat<T>(self, other: T) -> dsl::Concat<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Concat::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL binary `LIKE` expression.
    ///
    /// This method is case sensitive. There is no case-insensitive
    /// equivalent as of PostgreSQL 14.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> Binary,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::insert_into;
    /// #
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE users (
    /// #         id INTEGER PRIMARY KEY,
    /// #         name BYTEA NOT NULL
    /// #     )").execute(connection).unwrap();
    /// #
    /// #     insert_into(users)
    /// #         .values(&vec![
    /// #             (id.eq(1), name.eq("Sean".as_bytes())),
    /// #             (id.eq(2), name.eq("Tess".as_bytes()))
    /// #         ])
    /// #         .execute(connection)
    /// #         .unwrap();
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.like(b"S%".to_vec()))
    ///     .load(connection);
    /// assert_eq!(Ok(vec![b"Sean".to_vec()]), starts_with_s);
    /// # }
    /// ```
    fn like<T>(self, other: T) -> dsl::Like<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Like::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL binary `LIKE` expression.
    ///
    /// This method is case sensitive. There is no case-insensitive
    /// equivalent as of PostgreSQL 14.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Integer,
    /// #         name -> Binary,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::*;
    /// #     use diesel::insert_into;
    /// #
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE users (
    /// #         id INTEGER PRIMARY KEY,
    /// #         name BYTEA NOT NULL
    /// #     )").execute(connection).unwrap();
    /// #
    /// #     insert_into(users)
    /// #         .values(&vec![
    /// #             (id.eq(1), name.eq("Sean".as_bytes())),
    /// #             (id.eq(2), name.eq("Tess".as_bytes()))
    /// #         ])
    /// #         .execute(connection)
    /// #         .unwrap();
    /// #
    /// let starts_with_s = users
    ///     .select(name)
    ///     .filter(name.not_like(b"S%".to_vec()))
    ///     .load(connection);
    /// assert_eq!(Ok(vec![b"Tess".to_vec()]), starts_with_s);
    /// # }
    /// ```
    fn not_like<T>(self, other: T) -> dsl::NotLike<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(NotLike::new(self, other.as_expression()))
    }
}

#[doc(hidden)]
impl<T> PgBinaryExpressionMethods for T
where
    T: Expression,
    T::SqlType: BinaryOrNullableBinary,
{
}

pub(in crate::pg) mod private {
    use crate::sql_types::{
        AllAreNullable, Array, Binary, Cidr, Inet, Integer, Json, Jsonb, MaybeNullableType,
        Multirange, Nullable, OneIsNullable, Range, Record, SingleValue, SqlType, Text,
    };
    use crate::{Expression, IntoSql};

    /// Marker trait used to implement `ArrayExpressionMethods` on the appropriate
    /// types. Once coherence takes associated types into account, we can remove
    /// this trait.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Array<_>` nor `diesel::sql_types::Nullable<Array<_>>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait ArrayOrNullableArray {
        type Inner;
    }

    impl<T> ArrayOrNullableArray for Array<T> {
        type Inner = T;
    }
    impl<T> ArrayOrNullableArray for Nullable<Array<T>> {
        type Inner = T;
    }

    /// Marker trait used to implement `PgNetExpressionMethods` on the appropriate types.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Inet`, `diesel::sql_types::Cidr`, `diesel::sql_types::Nullable<Inet>` nor `diesel::sql_types::Nullable<Cidr>",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait InetOrCidr {}

    impl InetOrCidr for Inet {}
    impl InetOrCidr for Cidr {}
    impl InetOrCidr for Nullable<Inet> {}
    impl InetOrCidr for Nullable<Cidr> {}

    /// Marker trait used to implement `PgTextExpressionMethods` on the appropriate
    /// types. Once coherence takes associated types into account, we can remove
    /// this trait.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Text` nor `diesel::sql_types::Nullable<Text>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextOrNullableText {}

    impl TextOrNullableText for Text {}
    impl TextOrNullableText for Nullable<Text> {}

    /// Marker trait used to extract the inner type
    /// of our `Range<T>` and `Multirange<T>` sql type, used to implement `PgRangeExpressionMethods`
    pub trait RangeOrMultirange: SqlType + SingleValue {
        type Inner: SingleValue;
    }

    impl<ST> RangeOrMultirange for Range<ST>
    where
        Self: 'static,
        ST: SingleValue,
    {
        type Inner = ST;
    }

    impl<ST> RangeOrMultirange for Multirange<ST>
    where
        Self: 'static,
        ST: SingleValue,
    {
        type Inner = ST;
    }

    /// Marker trait used to implement `PgRangeExpressionMethods` on the appropriate
    /// types. Once coherence takes associated types into account, we can remove
    /// this trait.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Nullable<Range<_>>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait RangeOrNullableRange {
        type Inner: SingleValue;
    }

    impl<ST: SingleValue> RangeOrNullableRange for Range<ST> {
        type Inner = ST;
    }
    impl<ST: SingleValue> RangeOrNullableRange for Nullable<Range<ST>> {
        type Inner = ST;
    }

    /// Marker trait used to implement `PgRangeExpressionMethods` on the appropriate
    /// types. Once coherence takes associated types into account, we can remove
    /// this trait.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Nullable<Range<_>>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait MultirangeOrNullableMultirange {
        type Inner: SingleValue;
        type Range: SingleValue;
    }

    impl<ST: SingleValue> MultirangeOrNullableMultirange for Multirange<ST> {
        type Inner = ST;
        type Range = Range<ST>;
    }
    impl<ST: SingleValue> MultirangeOrNullableMultirange for Nullable<Multirange<ST>> {
        type Inner = ST;
        type Range = Nullable<Range<ST>>;
    }

    /// Marker trait used to implement `PgRangeExpressionMethods` on the appropriate
    /// types. Once coherence takes associated types into account, we can remove
    /// this trait.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Range<_>` nor `diesel::sql_types::Multirange<_>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait MultirangeOrRangeMaybeNullable {
        type Inner: SingleValue;
    }

    impl<ST: SingleValue> MultirangeOrRangeMaybeNullable for Range<ST> {
        type Inner = ST;
    }
    impl<ST: SingleValue> MultirangeOrRangeMaybeNullable for Nullable<Range<ST>> {
        type Inner = ST;
    }
    impl<ST: SingleValue> MultirangeOrRangeMaybeNullable for Multirange<ST> {
        type Inner = ST;
    }
    impl<ST: SingleValue> MultirangeOrRangeMaybeNullable for Nullable<Multirange<ST>> {
        type Inner = ST;
    }

    /// Marker trait used to implement `PgJsonbExpressionMethods` on the appropriate types.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Jsonb` nor `diesel::sql_types::Nullable<Jsonb>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait JsonbOrNullableJsonb {}

    impl JsonbOrNullableJsonb for Jsonb {}
    impl JsonbOrNullableJsonb for Nullable<Jsonb> {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Json` nor `diesel::sql_types::Nullable<Json>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait JsonOrNullableJson {}

    impl JsonOrNullableJson for Json {}
    impl JsonOrNullableJson for Nullable<Json> {}

    /// A trait that describes valid json indices used by postgresql
    pub trait JsonRemoveIndex {
        /// The Expression node created by this index type
        type Expression: Expression;

        /// Convert a index value into the corresponding index expression
        fn into_json_index_expression(self) -> Self::Expression;
    }

    impl<'a> JsonRemoveIndex for &'a str {
        type Expression = crate::dsl::AsExprOf<&'a str, crate::sql_types::Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonRemoveIndex for String {
        type Expression = crate::dsl::AsExprOf<String, crate::sql_types::Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonRemoveIndex for Vec<String> {
        type Expression = crate::dsl::AsExprOf<Self, Array<Text>>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Array<Text>>()
        }
    }

    impl JsonRemoveIndex for Vec<&str> {
        type Expression = crate::dsl::AsExprOf<Self, Array<Text>>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Array<Text>>()
        }
    }

    impl<'a> JsonRemoveIndex for &'a [&'a str] {
        type Expression = crate::dsl::AsExprOf<Self, Array<Text>>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Array<Text>>()
        }
    }

    impl JsonRemoveIndex for i32 {
        type Expression = crate::dsl::AsExprOf<i32, crate::sql_types::Int4>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<crate::sql_types::Int4>()
        }
    }

    impl<T> JsonRemoveIndex for T
    where
        T: Expression,
        T::SqlType: TextArrayOrTextOrInteger,
    {
        type Expression = Self;

        fn into_json_index_expression(self) -> Self::Expression {
            self
        }
    }

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Text`, `diesel::sql_types::Integer` nor `diesel::sql_types::Array<Text>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextArrayOrTextOrInteger {}

    impl TextArrayOrTextOrInteger for Array<Text> {}
    impl TextArrayOrTextOrInteger for Text {}
    impl TextArrayOrTextOrInteger for Integer {}

    /// Marker trait used to implement `PgAnyJsonExpressionMethods` on the appropriate types.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Json`, `diesel::sql_types::Jsonb`, `diesel::sql_types::Nullable<Json>` nor `diesel::sql_types::Nullable<Jsonb>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait JsonOrNullableJsonOrJsonbOrNullableJsonb {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Json {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Nullable<Json> {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Jsonb {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Nullable<Jsonb> {}

    pub trait JsonIndex {
        type Expression: Expression;

        fn into_json_index_expression(self) -> Self::Expression;
    }

    impl<'a> JsonIndex for &'a str {
        type Expression = crate::dsl::AsExprOf<&'a str, crate::sql_types::Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonIndex for String {
        type Expression = crate::dsl::AsExprOf<String, crate::sql_types::Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonIndex for i32 {
        type Expression = crate::dsl::AsExprOf<i32, crate::sql_types::Int4>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<crate::sql_types::Int4>()
        }
    }

    impl<T> JsonIndex for T
    where
        T: Expression,
        T::SqlType: TextOrInteger,
    {
        type Expression = Self;

        fn into_json_index_expression(self) -> Self::Expression {
            self
        }
    }

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Text` nor `diesel::sql_types::Integer`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextOrInteger {}
    impl TextOrInteger for Text {}
    impl TextOrInteger for Integer {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Binary` nor `diesel::sql_types::Nullable<Binary>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait BinaryOrNullableBinary {}

    impl BinaryOrNullableBinary for Binary {}
    impl BinaryOrNullableBinary for Nullable<Binary> {}

    pub trait MaybeNullableValue<T>: SingleValue {
        type Out: SingleValue;
    }

    impl<T, O> MaybeNullableValue<O> for T
    where
        T: SingleValue,
        T::IsNull: MaybeNullableType<O>,
        <T::IsNull as MaybeNullableType<O>>::Out: SingleValue,
    {
        type Out = <T::IsNull as MaybeNullableType<O>>::Out;
    }

    pub trait CombinedNullableValue<O, Out>: SingleValue {
        type Out: SingleValue;
    }

    impl<T, O, Out> CombinedNullableValue<O, Out> for T
    where
        T: SingleValue,
        O: SingleValue,
        T::IsNull: OneIsNullable<O::IsNull>,
        <T::IsNull as OneIsNullable<O::IsNull>>::Out: MaybeNullableType<Out>,
        <<T::IsNull as OneIsNullable<O::IsNull>>::Out as MaybeNullableType<Out>>::Out: SingleValue,
    {
        type Out = <<T::IsNull as OneIsNullable<O::IsNull>>::Out as MaybeNullableType<Out>>::Out;
    }

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `Array<Text>`, `Array<Nullable<Text>>`,\
                   `Nullable<Array<Text>>` nor `diesel::sql_types::Nullable<Array<Nullable<Text>>>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextArrayOrNullableTextArray {}

    impl TextArrayOrNullableTextArray for Array<Text> {}
    impl TextArrayOrNullableTextArray for Array<Nullable<Text>> {}
    impl TextArrayOrNullableTextArray for Nullable<Array<Text>> {}
    impl TextArrayOrNullableTextArray for Nullable<Array<Nullable<Text>>> {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `Record<T>` nor `Nullable<Record<T>>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait RecordOrNullableRecord {}

    impl<T> RecordOrNullableRecord for Record<T> {}
    impl<T> RecordOrNullableRecord for Nullable<Record<T>> {}

    pub trait CombinedAllNullableValue<O, Out>: SingleValue {
        type Out: SingleValue;
    }

    impl<T, O, Out> CombinedAllNullableValue<O, Out> for T
    where
        T: SingleValue,
        O: SingleValue,
        T::IsNull: AllAreNullable<O::IsNull>,
        <T::IsNull as AllAreNullable<O::IsNull>>::Out: MaybeNullableType<Out>,
        <<T::IsNull as AllAreNullable<O::IsNull>>::Out as MaybeNullableType<Out>>::Out: SingleValue,
    {
        type Out = <<T::IsNull as AllAreNullable<O::IsNull>>::Out as MaybeNullableType<Out>>::Out;
    }
}
