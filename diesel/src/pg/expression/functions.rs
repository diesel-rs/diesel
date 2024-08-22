//! PostgreSQL specific functions

use super::expression_methods::InetOrCidr;
use super::expression_methods::RangeHelper;
use crate::expression::functions::define_sql_function;
use crate::pg::expression::expression_methods::ArrayOrNullableArray;
use crate::pg::expression::expression_methods::MultirangeOrRangeMaybeNullable;
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
    /// Returns the lower bound of the range
    ///
    /// If the range is empty or has no lower bound, it returns NULL.
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
    /// # use diesel::pg::sql_types::{Range, Multirange};
    /// # use diesel::dsl::lower;
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let int = diesel::select(lower::<Range<_>,  _>(1..2)).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(Some(1), int);
    ///
    /// let int = diesel::select(lower::<Range<_>, _>(..2)).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(lower::<Nullable<Range<_>>, _>(None::<std::ops::Range<i32>>)).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(lower::<Multirange<_>, _>(vec![5..7])).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(Some(5), int);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn lower<R: MultirangeOrRangeMaybeNullable + SingleValue>(range: R) -> Nullable<R::Inner>;
}

define_sql_function! {
    /// Returns the upper bound of the range
    ///
    /// If the range is empty or has no upper bound, it returns NULL.
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
    /// # use diesel::pg::sql_types::{Range, Multirange};
    /// # use diesel::dsl::upper;
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let int = diesel::select(upper::<Range<_>,  _>(1..2)).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(Some(2), int);
    ///
    /// let int = diesel::select(upper::<Range<_>, _>(1..)).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(upper::<Nullable<Range<_>>, _>(None::<std::ops::Range<i32>>)).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(upper::<Multirange<_>, _>(vec![5..7])).get_result::<Option<i32>>(connection)?;
    /// assert_eq!(Some(7), int);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn upper<R: MultirangeOrRangeMaybeNullable + SingleValue>(range: R) -> Nullable<R::Inner>;
}

define_sql_function! {
    /// Returns true if the range is empty
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
    /// # use diesel::pg::sql_types::{Range, Multirange};
    /// # use diesel::dsl::isempty;
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let int = diesel::select(isempty::<Range<Integer>,  _>(1..5)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    ///
    /// let int = diesel::select(isempty::<Range<Integer>, _>(1..1)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(true), int);
    ///
    /// let int = diesel::select(isempty::<Nullable<Range<Integer>>, _>(None::<std::ops::Range<i32>>)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(isempty::<Multirange<Integer>, _>(vec![5..7])).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn isempty<R: MultirangeOrRangeMaybeNullable + SingleValue>(range: R) -> Nullable<Bool>;
}

define_sql_function! {
    /// Returns true if the range's lower bound is inclusive
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
    /// # use diesel::pg::sql_types::{Range, Multirange};
    /// # use diesel::dsl::lower_inc;
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let int = diesel::select(lower_inc::<Range<Integer>,  _>(1..5)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(true), int);
    ///
    /// let int = diesel::select(lower_inc::<Range<Integer>, _>(..5)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    ///
    /// let int = diesel::select(lower_inc::<Nullable<Range<Integer>>, _>(None::<std::ops::Range<i32>>)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(lower_inc::<Multirange<Integer>, _>(vec![5..7])).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(true), int);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn lower_inc<R: MultirangeOrRangeMaybeNullable + SingleValue>(range: R) -> Nullable<Bool>;
}

define_sql_function! {
    /// Returns true if the range's upper bound is inclusive
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
    /// # use diesel::pg::sql_types::{Range, Multirange};
    /// # use diesel::dsl::upper_inc;
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let int = diesel::select(upper_inc::<Range<Integer>,  _>(1..5)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    ///
    /// let int = diesel::select(upper_inc::<Nullable<Range<Integer>>, _>(None::<std::ops::Range<i32>>)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(upper_inc::<Multirange<Integer>, _>(vec![5..7])).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn upper_inc<R: MultirangeOrRangeMaybeNullable + SingleValue>(range: R) -> Nullable<Bool>;
}

define_sql_function! {
    /// Returns true if the range's lower bound is unbounded
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
    /// # use diesel::pg::sql_types::{Range, Multirange};
    /// # use diesel::dsl::lower_inf;
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let int = diesel::select(lower_inf::<Range<Integer>,  _>(1..5)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    ///
    /// let int = diesel::select(lower_inf::<Range<Integer>,  _>(..5)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(true), int);
    ///
    /// let int = diesel::select(lower_inf::<Nullable<Range<Integer>>, _>(None::<std::ops::Range<i32>>)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(lower_inf::<Multirange<Integer>, _>(vec![5..7])).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn lower_inf<R: MultirangeOrRangeMaybeNullable + SingleValue>(range: R) -> Nullable<Bool>;
}

define_sql_function! {
    /// Returns true if the range's upper bound is unbounded
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
    /// # use diesel::pg::sql_types::{Range, Multirange};
    /// # use diesel::dsl::upper_inf;
    /// #     use std::collections::Bound;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let int = diesel::select(upper_inf::<Range<Integer>,  _>(1..5)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    ///
    /// let int = diesel::select(upper_inf::<Range<Integer>,  _>(1..)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(true), int);
    ///
    /// let int = diesel::select(upper_inf::<Nullable<Range<Integer>>, _>(None::<std::ops::Range<i32>>)).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(None, int);
    ///
    /// let int = diesel::select(upper_inf::<Multirange<Integer>, _>(vec![5..7])).get_result::<Option<bool>>(connection)?;
    /// assert_eq!(Some(false), int);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn upper_inf<R: MultirangeOrRangeMaybeNullable + SingleValue>(range: R) -> Nullable<Bool>;
}

define_sql_function! {
    /// Returns the smallest range which includes both of the given ranges
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         first_versions -> Range<Integer>,
    /// #         second_versions -> Range<Integer>,
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
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, first_versions INT4RANGE NOT NULL, second_versions INT4RANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// use diesel::dsl::range_merge;
    /// diesel::insert_into(posts)
    ///     .values((
    ///        first_versions.eq((Bound::Included(5), Bound::Excluded(7))),
    ///        second_versions.eq((Bound::Included(6),Bound::Unbounded)),
    ///     )).execute(conn)?;
    ///
    /// let cool_posts = posts.select(range_merge(first_versions, second_versions))
    ///     .load::<(Bound<i32>, Bound<i32>)>(conn)?;
    /// assert_eq!(vec![(Bound::Included(5), Bound::Unbounded)], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn range_merge<T1: RangeHelper, T2: RangeHelper<Inner = T1::Inner>>(lhs: T1, rhs: T2) -> Range<T1::Inner>;
}

define_sql_function! {
    /// Returns range of integer
    ///
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
    /// Returns range of big ints
    ///
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
    /// Returns range of numeric values
    ///
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
    /// #     #[cfg(feature = "numeric")]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "numeric")]
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

define_sql_function! {
    /// Returns range of timestamps without timezone
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Tsrange,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     #[cfg(feature = "time")]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "time")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     use std::collections::Bound;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions TSRANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// use diesel::dsl::tsrange;
    /// use diesel::pg::sql_types::RangeBound;
    /// use time::{PrimitiveDateTime, macros::datetime};
    ///
    /// diesel::insert_into(posts)
    ///     .values(&[
    ///        versions.eq(tsrange(Some(datetime!(2020-01-01 0:00)), Some(datetime!(2021-01-01 0:00)), RangeBound::LowerBoundInclusiveUpperBoundInclusive)),
    ///        versions.eq(tsrange(None, Some(datetime!(2020-01-01 0:00)), RangeBound::LowerBoundInclusiveUpperBoundExclusive)),
    ///     ]).execute(conn)?;
    ///
    /// let cool_posts = posts.select(versions)
    ///     .load::<(Bound<PrimitiveDateTime>, Bound<PrimitiveDateTime>)>(conn)?;
    /// assert_eq!(vec![
    ///          (Bound::Included(datetime!(2020-01-01 0:00)), Bound::Included(datetime!(2021-01-01 0:00))),
    ///          (Bound::Unbounded, Bound::Excluded(datetime!(2020-01-01 0:00))),
    ///      ], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn tsrange(lower: Nullable<Timestamp>, upper: Nullable<Timestamp>, bound: RangeBoundEnum) -> Tsrange;
}

define_sql_function! {
    /// Returns range of timestamps with timezone
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Tstzrange,
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
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions TSTZRANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// use diesel::dsl::tstzrange;
    /// use diesel::pg::sql_types::RangeBound;
    /// use time::{OffsetDateTime, macros::datetime};
    ///
    /// diesel::insert_into(posts)
    ///     .values(&[
    ///        versions.eq(tstzrange(Some(datetime!(2020-01-01 0:00 UTC)), Some(datetime!(2021-01-01 0:00 -3)), RangeBound::LowerBoundInclusiveUpperBoundInclusive)),
    ///        versions.eq(tstzrange(None, Some(datetime!(2020-01-01 0:00 +2)), RangeBound::LowerBoundInclusiveUpperBoundExclusive)),
    ///     ]).execute(conn)?;
    ///
    /// let cool_posts = posts.select(versions)
    ///     .load::<(Bound<OffsetDateTime>, Bound<OffsetDateTime>)>(conn)?;
    /// assert_eq!(vec![
    ///          (Bound::Included(datetime!(2020-01-01 0:00 UTC)), Bound::Included(datetime!(2021-01-01 0:00 -3))),
    ///          (Bound::Unbounded, Bound::Excluded(datetime!(2020-01-01 0:00 +2))),
    ///      ], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn tstzrange(lower: Nullable<Timestamptz>, upper: Nullable<Timestamptz>, bound: RangeBoundEnum) -> Tstzrange;
}

define_sql_function! {
    /// Returns range of dates
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     posts {
    /// #         id -> Integer,
    /// #         versions -> Daterange,
    /// #     }
    /// # }
    /// #
    /// # fn main() {
    /// #     #[cfg(feature = "time")]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "time")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::posts::dsl::*;
    /// #     use std::collections::Bound;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS posts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE posts (id SERIAL PRIMARY KEY, versions DATERANGE NOT NULL)").execute(conn).unwrap();
    /// #
    /// use diesel::dsl::daterange;
    /// use diesel::pg::sql_types::RangeBound;
    /// use time::{Date, macros::date};
    ///
    /// diesel::insert_into(posts)
    ///     .values(&[
    ///        versions.eq(daterange(Some(date!(2020-01-01)), Some(date!(2021-01-01)), RangeBound::LowerBoundInclusiveUpperBoundInclusive)),
    ///        versions.eq(daterange(None, Some(date!(2020-01-01)), RangeBound::LowerBoundInclusiveUpperBoundExclusive)),
    ///     ]).execute(conn)?;
    ///
    /// let cool_posts = posts.select(versions)
    ///     .load::<(Bound<Date>, Bound<Date>)>(conn)?;
    /// assert_eq!(vec![
    ///          (Bound::Included(date!(2020-01-01)), Bound::Excluded(date!(2021-01-02))),
    ///          (Bound::Unbounded, Bound::Excluded(date!(2020-01-01))),
    ///      ], cool_posts);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    fn daterange(lower: Nullable<Date>, upper: Nullable<Date>, bound: RangeBoundEnum) -> Daterange;
}

#[cfg(feature = "postgres_backend")]
define_sql_function! {
    /// Append an element to the end of an array
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
    /// #     use diesel::dsl::array_append;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let ints = diesel::select(array_append::<Array<_>, Integer, _, _>(vec![1, 2], 3))
    ///     .get_result::<Vec<i32>>(connection)?;
    /// assert_eq!(vec![1, 2, 3], ints);
    ///
    /// let ints = diesel::select(array_append::<Array<_>, Nullable<Integer>, _, _>(vec![Some(1), Some(2)], None::<i32>))
    ///     .get_result::<Vec<Option<i32>>>(connection)?;
    /// assert_eq!(vec![Some(1), Some(2), None], ints);
    ///
    /// let ints = diesel::select(array_append::<Nullable<Array<_>>, Integer, _, _>(None::<Vec<i32>>, 3))
    ///     .get_result::<Vec<i32>>(connection)?;
    /// assert_eq!(vec![3], ints);
    ///
    /// let ints = diesel::select(array_append::<Nullable<Array<_>>, Nullable<Integer>, _, _>(None::<Vec<i32>>, None::<i32>))
    ///     .get_result::<Vec<Option<i32>>>(connection)?;
    /// assert_eq!(vec![None], ints);
    /// #     Ok(())
    /// # }
    /// ```
    fn array_append<Arr: ArrayOrNullableArray<Inner=T> + SingleValue, T: SingleValue>(a: Arr, e: T) -> Array<T>;
}

#[cfg(feature = "postgres_backend")]
define_sql_function! {
    /// Replace all occurrences of an element in an array with a given element
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
    /// #     use diesel::dsl::array_replace;
    /// #     use diesel::sql_types::{Nullable, Integer, Array};
    /// #     let connection = &mut establish_connection();
    /// let ints = diesel::select(array_replace::<Array<_>, Integer, _, _, _>(vec![1, 2, 5, 4], 5, 3))
    ///     .get_result::<Vec<i32>>(connection)?;
    /// assert_eq!(vec![1, 2, 3, 4], ints);
    ///
    /// let ints = diesel::select(array_replace::<Array<_>, Nullable<Integer>, _, _, _>(vec![Some(1), Some(2), Some(3)], Some(3), None::<i32>))
    ///     .get_result::<Vec<Option<i32>>>(connection)?;
    /// assert_eq!(vec![Some(1), Some(2), None], ints);
    ///
    /// let ints = diesel::select(array_replace::<Nullable<Array<_>>, Integer, _, _, _>(None::<Vec<i32>>, 1, 2))
    ///     .get_result::<Option<Vec<i32>>>(connection)?;
    /// 
    /// let ints = diesel::select(array_replace::<Nullable<Array<_>>, Nullable<Integer>, _, _, _>(None::<Vec<i32>>, None::<i32>, Some(1)))
    ///     .get_result::<Option<Vec<Option<i32>>>>(connection)?;
    /// assert_eq!(None, ints);
    /// #    Ok(())
    /// # }
    /// ```
    fn array_replace<Arr: ArrayOrNullableArray<Inner=T> + SingleValue, T: SingleValue>(a: Arr, e: T, r: T) -> Arr;
}
