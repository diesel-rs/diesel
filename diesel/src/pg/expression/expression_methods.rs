//! PostgreSQL specific expression methods

use super::operators::*;
use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::{AsExpression, Expression, IntoSql, TypedExpressionType};
use crate::sql_types::{Array, Cidr, Inet, Nullable, Range, SqlType, Text};

/// PostgreSQL specific methods which are present on all expressions.
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
    fn is_distinct_from<T>(self, other: T) -> dsl::IsDistinctFrom<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsDistinctFrom::new(self, other.as_expression()))
    }
}

impl<T: Expression> PgExpressionMethods for T {}

use super::date_and_time::{AtTimeZone, DateTimeLike};
use crate::sql_types::VarChar;

/// PostgreSQL specific methods present on timestamp expressions.
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
    /// #     connection.execute("CREATE TABLE timestamps (\"timestamp\"
    /// #         timestamp NOT NULL)")?;
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
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
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
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
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
    fn contains<T>(self, other: T) -> dsl::ArrayContains<Self, T>
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
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
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
    fn is_contained_by<T>(self, other: T) -> dsl::IsContainedBy<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsContainedBy::new(self, other.as_expression()))
    }
}

impl<T> PgArrayExpressionMethods for T
where
    T: Expression,
    T::SqlType: ArrayOrNullableArray,
{
}

#[doc(hidden)]
/// Marker trait used to implement `ArrayExpressionMethods` on the appropriate
/// types. Once coherence takes associated types into account, we can remove
/// this trait.
pub trait ArrayOrNullableArray {}

impl<T> ArrayOrNullableArray for Array<T> {}
impl<T> ArrayOrNullableArray for Nullable<Array<T>> {}

use crate::expression::operators::{Asc, Desc};
use crate::EscapeExpressionMethods;

/// PostgreSQL expression methods related to sorting.
///
/// This trait is only implemented for `Asc` and `Desc`. Although `.asc` is
/// implicit if no order is given, you will need to call `.asc()` explicitly in
/// order to call these methods.
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
    /// #     connection.execute("CREATE TABLE nullable_numbers (nullable_number INTEGER)")?;
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
    /// #     connection.execute("CREATE TABLE nullable_numbers (nullable_number INTEGER)")?;
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
}

#[doc(hidden)]
/// Marker trait used to implement `PgTextExpressionMethods` on the appropriate
/// types. Once coherence takes associated types into account, we can remove
/// this trait.
pub trait TextOrNullableText {}

impl TextOrNullableText for Text {}
impl TextOrNullableText for Nullable<Text> {}

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

#[doc(hidden)]
/// Marker trait used to extract the inner type
/// of our `Range<T>` sql type, used to implement `PgRangeExpressionMethods`
pub trait RangeHelper: SqlType {
    type Inner;
}

impl<ST> RangeHelper for Range<ST> {
    type Inner = ST;
}

/// PostgreSQL specific methods present on range expressions.
pub trait PgRangeExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `@>` expression.
    ///
    /// This operator returns whether a range contains an specific element
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
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions INT4RANGE NOT NULL)").unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(versions.eq((Bound::Included(5), Bound::Unbounded)))
    ///     .execute(conn)?;
    ///
    /// let cool_posts = posts.select(id)
    ///     .filter(versions.contains(42))
    ///     .load::<i32>(conn)?;
    /// assert_eq!(vec![1], cool_posts);
    ///
    /// let amazing_posts = posts.select(id)
    ///     .filter(versions.contains(1))
    ///     .load::<i32>(conn)?;
    /// assert!(amazing_posts.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> dsl::RangeContains<Self, T>
    where
        Self::SqlType: RangeHelper,
        <Self::SqlType as RangeHelper>::Inner: SqlType + TypedExpressionType,
        T: AsExpression<<Self::SqlType as RangeHelper>::Inner>,
    {
        Grouped(Contains::new(self, other.as_expression()))
    }
}

#[doc(hidden)]
/// Marker trait used to implement `PgRangeExpressionMethods` on the appropriate
/// types. Once coherence takes associated types into account, we can remove
/// this trait.
pub trait RangeOrNullableRange {}

impl<ST> RangeOrNullableRange for Range<ST> {}
impl<ST> RangeOrNullableRange for Nullable<Range<ST>> {}

impl<T> PgRangeExpressionMethods for T
where
    T: Expression,
    T::SqlType: RangeOrNullableRange,
{
}

/// PostgreSQL specific methods present between CIDR/INET expressions
pub trait PgNetExpressionMethods: Expression + Sized {
    /// Creates a PostgreSQL `>>` expression.
    ///
    /// This operator returns wether a subnet strictly contains another subnet or address.
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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
    /// This operator returns wether a subnet contains or is equal to another subnet.
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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
    /// This operator returns wether a subnet or address is strictly contained by another subnet.
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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
    fn is_contained_by<T>(self, other: T) -> dsl::IsContainedByNet<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(IsContainedByNet::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `>>=` expression.
    ///
    /// This operator returns wether a subnet is contained by or equal to another subnet.
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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
    fn is_contained_by_or_eq<T>(self, other: T) -> dsl::IsContainedByNetLoose<Self, T>
    where
        T: AsExpression<Inet>,
    {
        Grouped(IsContainedByNetLoose::new(self, other.as_expression()))
    }

    /// Creates a PostgreSQL `&&` expression.
    ///
    /// This operator returns wether a subnet contains or is contained by another subnet.
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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
    /// This operator substracts an address from an address to compute the distance between the two
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
    /// #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
    /// #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
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

#[doc(hidden)]
/// Marker trait used to implement `PgNetExpressionMethods` on the appropriate types.
pub trait InetOrCidr {}

impl InetOrCidr for Inet {}
impl InetOrCidr for Cidr {}
impl InetOrCidr for Nullable<Inet> {}
impl InetOrCidr for Nullable<Cidr> {}
