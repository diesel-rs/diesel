//! PostgreSQL specific expression methods

use super::operators::*;
use expression::{AsExpression, Expression};
use sql_types::{Array, Bigint, Cidr, Inet, Nullable, Text};

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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let distinct = users.select(id).filter(name.is_distinct_from("Sean"));
    /// let not_distinct = users.select(id).filter(name.is_not_distinct_from("Sean"));
    /// assert_eq!(Ok(2), distinct.first(&connection));
    /// assert_eq!(Ok(1), not_distinct.first(&connection));
    /// # }
    /// ```
    fn is_not_distinct_from<T>(self, other: T) -> IsNotDistinctFrom<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        IsNotDistinctFrom::new(self, other.as_expression())
    }

    /// Creates a PostgreSQL `IS DISTINCT FROM` expression.
    ///
    /// This behaves identically to the `!=` operator, except that `NULL` is
    /// treated as a normal value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     use schema::users::dsl::*;
    /// #     let connection = establish_connection();
    /// let distinct = users.select(id).filter(name.is_distinct_from("Sean"));
    /// let not_distinct = users.select(id).filter(name.is_not_distinct_from("Sean"));
    /// assert_eq!(Ok(2), distinct.first(&connection));
    /// assert_eq!(Ok(1), not_distinct.first(&connection));
    /// # }
    /// ```
    fn is_distinct_from<T>(self, other: T) -> IsDistinctFrom<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        IsDistinctFrom::new(self, other.as_expression())
    }
}

impl<T: Expression> PgExpressionMethods for T {}

use super::date_and_time::{AtTimeZone, DateTimeLike};
use sql_types::VarChar;

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
    /// # #[macro_use] extern crate diesel;
    /// # #[cfg(feature = "chrono")]
    /// # extern crate chrono;
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
    /// #     use timestamps::dsl::*;
    /// #     use chrono::*;
    /// #     let connection = establish_connection();
    /// #     connection.execute("CREATE TABLE timestamps (\"timestamp\"
    /// #         timestamp NOT NULL)")?;
    /// let christmas_morning = NaiveDate::from_ymd(2017, 12, 25)
    ///     .and_hms(8, 0, 0);
    /// diesel::insert_into(timestamps)
    ///     .values(timestamp.eq(christmas_morning))
    ///     .execute(&connection)?;
    ///
    /// let utc_time = timestamps
    ///     .select(timestamp.at_time_zone("UTC"))
    ///     .first(&connection)?;
    /// assert_eq!(christmas_morning, utc_time);
    ///
    /// let eastern_time = timestamps
    ///     .select(timestamp.at_time_zone("EST"))
    ///     .first(&connection)?;
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
    fn at_time_zone<T>(self, timezone: T) -> AtTimeZone<Self, T::Expression>
    where
        T: AsExpression<VarChar>,
    {
        AtTimeZone::new(self, timezone.as_expression())
    }
}

impl<T: Expression> PgTimestampExpressionMethods for T where T::SqlType: DateTimeLike {}

/// PostgreSQL specific methods present on array expressions.
pub trait PgArrayExpressionMethods<ST>: Expression<SqlType = Array<ST>> + Sized {
    /// Creates a PostgreSQL `&&` expression.
    ///
    /// This operator returns whether two arrays have common elements.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
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
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(&vec![
    ///         tags.eq(vec!["cool", "awesome"]),
    ///         tags.eq(vec!["awesome", "great"]),
    ///         tags.eq(vec!["cool", "great"]),
    ///     ])
    ///     .execute(&conn)?;
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.overlaps_with(vec!["horrid", "cool"]))
    ///     .load::<i32>(&conn)?;
    /// assert_eq!(vec![1, 3], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.overlaps_with(vec!["cool", "great"]))
    ///     .load::<i32>(&conn)?;
    /// assert_eq!(vec![1, 2, 3], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.overlaps_with(vec!["horrid"]))
    ///     .load::<i32>(&conn)?;
    /// assert!(data.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    fn overlaps_with<T>(self, other: T) -> OverlapsWith<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        OverlapsWith::new(self, other.as_expression())
    }

    /// Creates a PostgreSQL `@>` expression.
    ///
    /// This operator returns whether an array contains another array.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
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
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(tags.eq(vec!["cool", "awesome"]))
    ///     .execute(&conn)?;
    ///
    /// let cool_posts = posts.select(id)
    ///     .filter(tags.contains(vec!["cool"]))
    ///     .load::<i32>(&conn)?;
    /// assert_eq!(vec![1], cool_posts);
    ///
    /// let amazing_posts = posts.select(id)
    ///     .filter(tags.contains(vec!["cool", "amazing"]))
    ///     .load::<i32>(&conn)?;
    /// assert!(amazing_posts.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    fn contains<T>(self, other: T) -> Contains<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        Contains::new(self, other.as_expression())
    }

    /// Creates a PostgreSQL `<@` expression.
    ///
    /// This operator returns whether an array is contained by another array.
    /// `foo.contains(bar)` is the same as `bar.is_contained_by(foo)`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
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
    /// #     let conn = establish_connection();
    /// #     conn.execute("DROP TABLE IF EXISTS posts").unwrap();
    /// #     conn.execute("CREATE TABLE posts (id SERIAL PRIMARY KEY, tags TEXT[] NOT NULL)").unwrap();
    /// #
    /// diesel::insert_into(posts)
    ///     .values(tags.eq(vec!["cool", "awesome"]))
    ///     .execute(&conn)?;
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.is_contained_by(vec!["cool", "awesome", "amazing"]))
    ///     .load::<i32>(&conn)?;
    /// assert_eq!(vec![1], data);
    ///
    /// let data = posts.select(id)
    ///     .filter(tags.is_contained_by(vec!["cool"]))
    ///     .load::<i32>(&conn)?;
    /// assert!(data.is_empty());
    /// #     Ok(())
    /// # }
    /// ```
    fn is_contained_by<T>(self, other: T) -> IsContainedBy<Self, T::Expression>
    where
        T: AsExpression<Self::SqlType>,
    {
        IsContainedBy::new(self, other.as_expression())
    }
}

impl<T, ST> PgArrayExpressionMethods<ST> for T where T: Expression<SqlType = Array<ST>> {}

use expression::operators::{Asc, Desc};

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
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE nullable_numbers (nullable_number INTEGER)")?;
    /// diesel::insert_into(nullable_numbers)
    ///     .values(&vec![
    ///         nullable_number.eq(None),
    ///         nullable_number.eq(Some(1)),
    ///         nullable_number.eq(Some(2)),
    ///     ])
    ///     .execute(&connection)?;
    ///
    /// let asc_default_nulls = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.asc())
    ///     .load(&connection)?;
    /// assert_eq!(vec![Some(1), Some(2), None], asc_default_nulls);
    ///
    /// let asc_nulls_first = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.asc().nulls_first())
    ///     .load(&connection)?;
    /// assert_eq!(vec![None, Some(1), Some(2)], asc_nulls_first);
    /// #     Ok(())
    /// # }
    /// ```
    fn nulls_first(self) -> NullsFirst<Self> {
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
    /// # #[macro_use] extern crate diesel;
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
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE nullable_numbers (nullable_number INTEGER)")?;
    /// diesel::insert_into(nullable_numbers)
    ///     .values(&vec![
    ///         nullable_number.eq(None),
    ///         nullable_number.eq(Some(1)),
    ///         nullable_number.eq(Some(2)),
    ///     ])
    ///     .execute(&connection)?;
    ///
    /// let desc_default_nulls = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.desc())
    ///     .load(&connection)?;
    /// assert_eq!(vec![None, Some(2), Some(1)], desc_default_nulls);
    ///
    /// let desc_nulls_last = nullable_numbers.select(nullable_number)
    ///     .order(nullable_number.desc().nulls_last())
    ///     .load(&connection)?;
    /// assert_eq!(vec![Some(2), Some(1), None], desc_nulls_last);
    /// #     Ok(())
    /// # }
    /// ```
    fn nulls_last(self) -> NullsLast<Self> {
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
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// let starts_with_s = animals
    ///     .select(species)
    ///     .filter(name.ilike("s%").or(species.ilike("s%")))
    ///     .get_results::<String>(&connection)?;
    /// assert_eq!(vec!["spider"], starts_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn ilike<T: AsExpression<Text>>(self, other: T) -> ILike<Self, T::Expression> {
        ILike::new(self.as_expression(), other.as_expression())
    }

    /// Creates a PostgreSQL `NOT ILIKE` expression
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// let doesnt_start_with_s = animals
    ///     .select(species)
    ///     .filter(name.not_ilike("s%").and(species.not_ilike("s%")))
    ///     .get_results::<String>(&connection)?;
    /// assert_eq!(vec!["dog"], doesnt_start_with_s);
    /// #     Ok(())
    /// # }
    /// ```
    fn not_ilike<T: AsExpression<Text>>(self, other: T) -> NotILike<Self, T::Expression> {
        NotILike::new(self.as_expression(), other.as_expression())
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

/// PostgreSQL specific methods present between CIDR/INET expressions
pub trait PgNetExpressionMethods: Expression + Sized {
    /**
     * Creates a PostgreSQL `>>` expression.
     *
     * This operator returns wether a subnet strictly contains another subnet or address.
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
     *                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
     *     .execute(&conn)?;
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.contains(IpNetwork::from_str("10.0.2.5").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1, 2], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.contains(IpNetwork::from_str("10.0.2.5/24").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![2], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.contains(IpNetwork::from_str("10.0.3.31").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![2], my_hosts);
     * #     Ok(())
     * # }
     * ```
     */
    fn contains<T>(self, other: T) -> ContainsNet<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        ContainsNet::new(self, other.as_expression())
    }

    /**
     * Creates a PostgreSQL `>>=` expression.
     *
     * This operator returns wether a subnet contains or is equal to another subnet.
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
     *                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
     *     .execute(&conn)?;
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.contains_or_eq(IpNetwork::from_str("10.0.2.5").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1, 2], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.contains_or_eq(IpNetwork::from_str("10.0.2.5/24").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1, 2], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.contains_or_eq(IpNetwork::from_str("10.0.3.31").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![2], my_hosts);
     * #     Ok(())
     * # }
     * ```
     */
    fn contains_or_eq<T>(self, other: T) -> ContainsNetLoose<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        ContainsNetLoose::new(self, other.as_expression())
    }

    /**
     * Creates a PostgreSQL `<<` expression.
     *
     * This operator returns wether a subnet or address is strictly contained by another subnet.
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
     *                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
     *     .execute(&conn)?;
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.is_contained_by(IpNetwork::from_str("10.0.2.5/24").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(my_hosts.len(), 0);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.is_contained_by(IpNetwork::from_str("10.0.3.31/23").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.is_contained_by(IpNetwork::from_str("10.0.3.31/22").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1, 2], my_hosts);
     * #     Ok(())
     * # }
     * ```
     */
    fn is_contained_by<T>(self, other: T) -> IsContainedByNet<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        IsContainedByNet::new(self, other.as_expression())
    }

    /**
     * Creates a PostgreSQL `>>=` expression.
     *
     * This operator returns wether a subnet is contained by or equal to another subnet.
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
     *                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
     *     .execute(&conn)?;
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.is_contained_by_or_eq(IpNetwork::from_str("10.0.2.5/24").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.is_contained_by_or_eq(IpNetwork::from_str("10.0.3.31/23").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1, 2], my_hosts);
     * #     Ok(())
     * # }
     * ```
     */
    fn is_contained_by_or_eq<T>(self, other: T) -> IsContainedByNetLoose<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        IsContainedByNetLoose::new(self, other.as_expression())
    }

    /**
     * Creates a PostgreSQL `&&` expression.
     *
     * This operator returns wether a subnet contains or is contained by another subnet.
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3/24").unwrap()),
     *                  address.eq(IpNetwork::from_str("10.0.3.4/23").unwrap())])
     *     .execute(&conn)?;
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.overlaps_with(IpNetwork::from_str("10.0.2.5/24").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1, 2], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.overlaps_with(IpNetwork::from_str("10.0.3.31/24").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![2], my_hosts);
     *
     * let my_hosts = hosts.select(id)
     *     .filter(address.overlaps_with(IpNetwork::from_str("10.0.3.31/23").unwrap()))
     *     .load::<i32>(&conn)?;
     * assert_eq!(vec![1, 2], my_hosts);
     * #     Ok(())
     * # }
     * ```
     */
    fn overlaps_with<T>(self, other: T) -> OverlapsWith<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        OverlapsWith::new(self, other.as_expression())
    }

    /**
     * Creates a PostgreSQL `&` expression.
     *
     * This operator computes the bitwise AND between two network addresses.
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3").unwrap())])
     *     .execute(&conn)?;
     *
     * let addr = hosts
     *     .select(address.and(IpNetwork::from_str("0.0.0.255").unwrap()))
     *     .first::<IpNetwork>(&conn)?;
     * assert_eq!(addr, IpNetwork::from_str("0.0.0.3").unwrap());
     * #     Ok(())
     * # }
     * ```
     */
    fn and<T>(self, other: T) -> AndNet<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        AndNet::new(self, other.as_expression())
    }

    /**
     * Creates a PostgreSQL `|` expression.
     *
     * This operator computes the bitwise OR between two network addresses.
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3").unwrap())])
     *     .execute(&conn)?;
     *
     * let addr = hosts
     *     .select(address.or(IpNetwork::from_str("0.0.0.255").unwrap()))
     *     .first::<IpNetwork>(&conn)?;
     * assert_eq!(addr, IpNetwork::from_str("10.0.2.255").unwrap());
     * #     Ok(())
     * # }
     * ```
     */
    fn or<T>(self, other: T) -> OrNet<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        OrNet::new(self, other.as_expression())
    }
}

impl<T> PgNetExpressionMethods for T where T: Expression, T::SqlType: InetOrCidr {}

/// PostgreSQL specific methods present between CIDR/INET expressions and Bigint expressions
pub trait PgNetAddExpressionMethods: Expression + Sized {
    /**
     * Creates a PostgreSQL `+` expression.
     *
     * This operator adds an offset to an address
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.3").unwrap())])
     *     .execute(&conn)?;
     *
     * let addr = hosts
     *     .select(address.add(10))
     *     .first::<IpNetwork>(&conn)?;
     * assert_eq!(addr, IpNetwork::from_str("10.0.2.13").unwrap());
     * #     Ok(())
     * # }
     * ```
     */
    fn add<T>(self, other: T) -> AddNet<Self, T::Expression>
    where
        T: AsExpression<Bigint>,
    {
        AddNet::new(self, other.as_expression())
    }
}

impl<T> PgNetAddExpressionMethods for T where T: Expression, T::SqlType: InetOrCidr {}

/// PostgreSQL specific methods present between CIDR/INET expressions and Bigint expression
pub trait PgNetSubExpressionMethods: Expression + Sized {
    /**
     * Creates a PostgreSQL `-` expression.
     *
     * This operator substracts an offset from an address
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.53").unwrap())])
     *     .execute(&conn)?;
     *
     * let addr = hosts
     *     .select(address.sub(10))
     *     .first::<IpNetwork>(&conn)?;
     * assert_eq!(addr, IpNetwork::from_str("10.0.2.43").unwrap());
     * #     Ok(())
     * # }
     * ```
     */
    fn sub<T>(self, other: T) -> SubstractNet<Self, T::Expression>
    where
        T: AsExpression<Bigint>,
    {
        SubstractNet::new(self, other.as_expression())
    }
}

impl<T> PgNetSubExpressionMethods for T where T: Expression, T::SqlType: InetOrCidr {}

/// PostgreSQL specific methods present between CIDR/INET expressions
pub trait PgNetDiffExpressionMethods: Expression + Sized {
    /**
     * Creates a PostgreSQL `-` expression.
     *
     * This operator substracts an address from an address to compute the distance between the two
     *
     * # Example
     *
     * ```rust
     * # #[macro_use] extern crate diesel;
     * # extern crate ipnetwork;
     * # include!("../../doctest_setup.rs");
     * #
     * # table! {
     * #     hosts {
     * #         id -> Integer,
     * #         address -> Inet,
     * #     }
     * # }
     * #
     * # fn main() {
     * #     run_test().unwrap();
     * # }
     * #
     * # fn run_test() -> QueryResult<()> {
     * #     use self::hosts::dsl::*;
     * #     use ipnetwork::IpNetwork;
     * #     use std::str::FromStr;
     * #     let conn = establish_connection();
     * #     conn.execute("DROP TABLE IF EXISTS hosts").unwrap();
     * #     conn.execute("CREATE TABLE hosts (id SERIAL PRIMARY KEY, address INET NOT NULL)").unwrap();
     * diesel::insert_into(hosts)
     *     .values(vec![address.eq(IpNetwork::from_str("10.0.2.53").unwrap())])
     *     .execute(&conn)?;
     *
     * let offset = hosts
     *     .select(address.diff(IpNetwork::from_str("10.0.2.42").unwrap()))
     *     .first::<i64>(&conn)?;
     * assert_eq!(offset, 11);
     * #     Ok(())
     * # }
     * ```
     */
    fn diff<T>(self, other: T) -> DifferenceNet<Self, T::Expression>
    where
        T: AsExpression<Inet>,
    {
        DifferenceNet::new(self, other.as_expression())
    }
}

impl<T> PgNetDiffExpressionMethods for T where T: Expression, T::SqlType: InetOrCidr {}

#[doc(hidden)]
/// Marker trait used to implement `PgNet*ExpressionMethods` on the appropriate types.
pub trait InetOrCidr {}

impl InetOrCidr for Inet {}
impl InetOrCidr for Cidr {}
