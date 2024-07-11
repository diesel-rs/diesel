//! PostgreSQL specific functions

use super::expression_methods::InetOrCidr;
use super::expression_methods::RangeHelper;
use crate::expression::functions::define_sql_function;
use crate::sql_types::*;

define_sql_function! {
    /// Creates an abbreviated display format as text.
    #[cfg(feature = "postgres_backend")]
    fn abbrev<T: InetOrCidr + SingleValue>(addr: T) -> Text;
}
define_sql_function! {
    /// Computes the broadcast address for the address's network.
    #[cfg(feature = "postgres_backend")]
    fn broadcast<T: InetOrCidr + SingleValue>(addr: T) -> Inet;
}
define_sql_function! {
    /// Returns the address's family: 4 for IPv4, 6 for IPv6.
    #[cfg(feature = "postgres_backend")]
    fn family<T: InetOrCidr + SingleValue>(addr: T) -> Integer;
}
define_sql_function! {
    /// Returns the IP address as text, ignoring the netmask.
    #[cfg(feature = "postgres_backend")]
    fn host<T: InetOrCidr + SingleValue>(addr: T) -> Text;
}
define_sql_function! {
    /// Computes the host mask for the address's network.
    #[cfg(feature = "postgres_backend")]
    fn hostmask<T: InetOrCidr + SingleValue>(addr: T) -> Inet;
}
define_sql_function! {
    /// Computes the smallest network that includes both of the given networks.
    #[cfg(feature = "postgres_backend")]
    fn inet_merge<T: InetOrCidr + SingleValue, U: InetOrCidr + SingleValue>(a: T, b: U) -> Cidr;
}
define_sql_function! {
    /// Tests whether the addresses belong to the same IP family.
    #[cfg(feature = "postgres_backend")]
    fn inet_same_family<T: InetOrCidr + SingleValue, U: InetOrCidr + SingleValue>(a: T, b: U) -> Bool;
}
define_sql_function! {
    /// Returns the netmask length in bits.
    #[cfg(feature = "postgres_backend")]
    fn masklen<T: InetOrCidr + SingleValue>(addr: T) -> Integer;
}
define_sql_function! {
    /// Computes the network mask for the address's network.
    #[cfg(feature = "postgres_backend")]
    fn netmask<T: InetOrCidr + SingleValue>(addr: T) -> Inet;
}
define_sql_function! {
    /// Returns the network part of the address, zeroing out whatever is to the right of the
    /// netmask. (This is equivalent to casting the value to cidr.)
    #[cfg(feature = "postgres_backend")]
    fn network<T: InetOrCidr + SingleValue>(addr: T) -> Cidr;
}
define_sql_function! {
    /// Sets the netmask length for an inet or cidr value.
    /// For inet, the address part does not changes. For cidr, address bits to the right of the new
    /// netmask are set to zero.
    #[cfg(feature = "postgres_backend")]
    fn set_masklen<T: InetOrCidr + SingleValue>(addr: T, len: Integer) -> T;
}

define_sql_function! {
    /// Returns the lower bound of the range.
    /// if the range is empty or has no lower bound, it returns NULL.
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
    /// use diesel::dsl::lower;
    /// diesel::insert_into(posts)
    ///     .values(&[
    ///        versions.eq((Bound::Included(5), Bound::Included(7))),
    ///        versions.eq((Bound::Unbounded, Bound::Included(7)))
    ///     ]).execute(conn)?;
    ///
    /// let cool_posts = posts.select(lower(versions))
    ///     .load::<Option<i32>>(conn)?;
    /// assert_eq!(vec![Some(5), None], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn lower<T: RangeHelper>(range: T) -> Nullable<<T as RangeHelper>::Inner>;
}

define_sql_function! {
    /// Returns range of integer.
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Int4range,
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
    /// use diesel::dsl::int4range;
    /// use diesel::pg::sql_types::RangeBound;
    ///
    /// diesel::insert_into(posts)
    ///     .values(&[
    ///        versions.eq(int4range(Some(3), Some(5), RangeBound::LowerBoundInclusiveUpperBoundInclusive)),
    ///        versions.eq(int4range(None, Some(2), RangeBound::LowerBoundInclusiveUpperBoundExclusive)),
    ///     ]).execute(conn)?;
    ///
    /// let cool_posts = posts.select(versions)
    ///     .load::<(Bound<i32>, Bound<i32>)>(conn)?;
    /// assert_eq!(vec![
    ///          (Bound::Included(3), Bound::Excluded(6)), // Postgres cast this internally
    ///          (Bound::Unbounded, Bound::Excluded(2)),
    ///      ], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn int4range(lower: Nullable<Integer>, upper: Nullable<Integer>, bound: RangeBoundEnum) -> Int4range;
}

define_sql_function! {
    /// Returns range of integer.
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Int8range,
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
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions INT8RANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// use diesel::dsl::int8range;
    /// use diesel::pg::sql_types::RangeBound;
    ///
    /// diesel::insert_into(posts)
    ///     .values(&[
    ///        versions.eq(int8range(Some(3), Some(5), RangeBound::LowerBoundInclusiveUpperBoundInclusive)),
    ///        versions.eq(int8range(None, Some(2), RangeBound::LowerBoundInclusiveUpperBoundExclusive)),
    ///     ]).execute(conn)?;
    ///
    /// let cool_posts = posts.select(versions)
    ///     .load::<(Bound<i64>, Bound<i64>)>(conn)?;
    /// assert_eq!(vec![
    ///          (Bound::Included(3), Bound::Excluded(6)), // Postgres cast this internally
    ///          (Bound::Unbounded, Bound::Excluded(2)),
    ///      ], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn int8range(lower: Nullable<BigInt>, upper: Nullable<BigInt>, bound: RangeBoundEnum) -> Int8range;
}

define_sql_function! {
    /// Returns range of number.
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Numrange,
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
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions NUMRANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// # use bigdecimal::BigDecimal;
    /// use diesel::dsl::numrange;
    /// use diesel::pg::sql_types::RangeBound;
    ///
    /// diesel::insert_into(posts)
    ///     .values(&[
    ///        versions.eq(numrange(Some(BigDecimal::from(3)), Some(BigDecimal::from(5)), RangeBound::LowerBoundInclusiveUpperBoundInclusive)),
    ///        versions.eq(numrange(None, Some(BigDecimal::from(2)), RangeBound::LowerBoundInclusiveUpperBoundExclusive)),
    ///     ]).execute(conn)?;
    ///
    /// let cool_posts = posts.select(versions)
    ///     .load::<(Bound<BigDecimal>, Bound<BigDecimal>)>(conn)?;
    /// assert_eq!(vec![
    ///          (Bound::Included(BigDecimal::from(3)), Bound::Included(BigDecimal::from(5))),
    ///          (Bound::Unbounded, Bound::Excluded(BigDecimal::from(2))),
    ///      ], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn numrange(lower: Nullable<Numeric>, upper: Nullable<Numeric>, bound: RangeBoundEnum) -> Numrange;
}
