//! PostgreSQL specific types

mod array;
#[doc(hidden)]
pub(in crate::pg) mod date_and_time;
#[doc(hidden)]
pub(in crate::pg) mod floats;
mod integers;
#[cfg(feature = "ipnet-address")]
mod ipnet_address;
#[cfg(feature = "serde_json")]
mod json;
mod json_path;
mod mac_addr;
mod mac_addr_8;
#[doc(hidden)]
pub(in crate::pg) mod money;
mod multirange;
#[cfg(feature = "network-address")]
mod network_address;
mod numeric;
mod primitives;
mod ranges;
mod record;
#[cfg(feature = "uuid")]
mod uuid;

/// PostgreSQL specific SQL types
///
/// Note: All types in this module can be accessed through `diesel::sql_types`
pub mod sql_types {
    use crate::query_builder::QueryId;
    use crate::sql_types::SqlType;

    /// The [`OID`] SQL type. This is a PostgreSQL specific type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`u32`]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`u32`]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [`u32`]: https://doc.rust-lang.org/nightly/std/primitive.u32.html
    /// [`OID`]: https://www.postgresql.org/docs/current/datatype-oid.html
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 26, array_oid = 1018))]
    pub struct Oid;

    /// The ["timestamp with time zone" SQL type][tz], which PostgreSQL abbreviates
    /// to `timestamptz`.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`PgTimestamp`]
    /// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
    /// - [`chrono::DateTime`] with `feature = "chrono"`
    /// - [`time::PrimitiveDateTime`] with `feature = "time"`
    /// - [`time::OffsetDateTime`] with `feature = "time"`
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`PgTimestamp`]
    /// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
    /// - [`chrono::DateTime`] with `feature = "chrono"`
    /// - [`time::PrimitiveDateTime`] with `feature = "time"`
    /// - [`time::OffsetDateTime`] with `feature = "time"`
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [`PgTimestamp`]: super::super::data_types::PgTimestamp
    /// [tz]: https://www.postgresql.org/docs/current/datatype-datetime.html
    #[cfg_attr(
        feature = "chrono",
        doc = " [`chrono::NaiveDateTime`]: chrono::naive::NaiveDateTime"
    )]
    #[cfg_attr(
        not(feature = "chrono"),
        doc = " [`chrono::NaiveDateTime`]: https://docs.rs/chrono/0.4.19/chrono/naive/struct.NaiveDateTime.html"
    )]
    #[cfg_attr(feature = "chrono", doc = " [`chrono::DateTime`]: chrono::DateTime")]
    #[cfg_attr(
        not(feature = "chrono"),
        doc = " [`chrono::DateTime`]: https://docs.rs/chrono/0.4.19/chrono/struct.DateTime.html"
    )]
    #[cfg_attr(
        feature = "time",
        doc = " [`time::PrimitiveDateTime`]: time::PrimitiveDateTime"
    )]
    #[cfg_attr(
        not(feature = "time"),
        doc = " [`time::PrimitiveDateTime`]: https://docs.rs/time/0.3.9/time/struct.PrimitiveDateTime.html"
    )]
    #[cfg_attr(
        feature = "time",
        doc = " [`time::OffsetDateTime`]: time::OffsetDateTime"
    )]
    #[cfg_attr(
        not(feature = "time"),
        doc = " [`time::OffsetDateTime`]: https://docs.rs/time/0.3.9/time/struct.OffsetDateTime.html"
    )]
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 1184, array_oid = 1185))]
    pub struct Timestamptz;

    /// The [`Array`] SQL type.
    ///
    /// This wraps another type to represent a SQL array of that type.
    /// Multidimensional arrays are not supported.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`Vec<T>`][Vec] for any `T` which implements `ToSql<ST>`
    /// - [`&[T]`][slice] for any `T` which implements `ToSql<ST>`
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`Vec<T>`][Vec] for any `T` which implements `ToSql<ST>`
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [Vec]: std::vec::Vec
    /// [slice]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
    /// [`Array`]: https://www.postgresql.org/docs/current/arrays.html
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[cfg(feature = "postgres_backend")]
    pub struct Array<ST: 'static>(ST);

    /// The [`Range`] SQL type.
    ///
    /// This wraps another type to represent a SQL range of that type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`(Bound<T>, Bound<T>)`][bound] for any `T` which implements `ToSql<ST>`.
    /// - [`Range<T>`][std::range] (aka `start..end`) for any `T` which implements `ToSql<ST>`.
    /// - [`RangeInclusive<T>`] (aka `start..=end`) for any `T` which implements `ToSql<ST>`.
    /// - [`RangeFrom<T>`] (aka `start..`) for any `T` which implements `ToSql<ST>`.
    /// - [`RangeTo<T>`] (aka `..end`) for any `T` which implements `ToSql<ST>`.
    /// - [`RangeToInclusive<T>`] (aka `..=end`) for any `T` which implements `ToSql<ST>`.
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`(Bound<T>, Bound<T>)`][bound] for any `T` which implements `FromSql<ST>`.
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [bound]: std::collections::Bound
    /// [std::range]: std::ops::Range
    /// [`RangeInclusive<T>`]: std::ops::RangeInclusive
    /// [`RangeFrom<T>`]: std::ops::RangeFrom
    /// [`RangeTo<T>`]: std::ops::RangeTo
    /// [`RangeToInclusive<T>`]: std::ops::RangeToInclusive
    /// [`Range`]: https://www.postgresql.org/docs/current/rangetypes.html
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[cfg(feature = "postgres_backend")]
    pub struct Range<ST: 'static>(ST);

    #[doc(hidden)]
    pub type Int4range = Range<crate::sql_types::Int4>;
    #[doc(hidden)]
    pub type Int8range = Range<crate::sql_types::Int8>;
    #[doc(hidden)]
    pub type Daterange = Range<crate::sql_types::Date>;
    #[doc(hidden)]
    pub type Numrange = Range<crate::sql_types::Numeric>;
    #[doc(hidden)]
    pub type Tsrange = Range<crate::sql_types::Timestamp>;
    #[doc(hidden)]
    pub type Tstzrange = Range<crate::sql_types::Timestamptz>;

    /// The [`Multirange`] SQL type.
    ///
    /// This wraps another type to represent a SQL range of that type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`Vec<T>`][Vec] for any `T` which `Range<T>` implements `ToSql<ST>`
    /// - [`&[T]`][slice] for any `T` which `Range<T>` implements `ToSql<ST>`
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`Vec<T>`][Vec] for any `T` which implements `ToSql<Range<ST>>`
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [Vec]: std::vec::Vec
    /// [slice]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
    /// [`Multirange`]: https://www.postgresql.org/docs/current/rangetypes.html
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[cfg(feature = "postgres_backend")]
    pub struct Multirange<ST: 'static>(ST);

    #[doc(hidden)]
    pub type Int4multirange = Multirange<crate::sql_types::Int4>;
    #[doc(hidden)]
    pub type Int8multirange = Multirange<crate::sql_types::Int8>;
    #[doc(hidden)]
    pub type Datemultirange = Multirange<crate::sql_types::Date>;
    #[doc(hidden)]
    pub type Nummultirange = Multirange<crate::sql_types::Numeric>;
    #[doc(hidden)]
    pub type Tsmultirange = Multirange<crate::sql_types::Timestamp>;
    #[doc(hidden)]
    pub type Tstzmultirange = Multirange<crate::sql_types::Timestamptz>;

    /// This is a wrapper for [`RangeBound`] to represent range bounds: '[]', '(]', '[)', '()',
    /// used in functions int4range, int8range, numrange, tsrange, tstzrange, daterange.
    #[derive(Debug, Clone, Copy, QueryId, SqlType)]
    #[cfg(feature = "postgres_backend")]
    #[diesel(postgres_type(name = "text"))]
    pub struct RangeBoundEnum;

    /// Represent postgres range bounds: '[]', '(]', '[)', '()',
    /// used in functions int4range, int8range, numrange, tsrange, tstzrange, daterange.
    #[derive(Debug, Clone, Copy, diesel_derives::AsExpression)]
    #[diesel(sql_type = RangeBoundEnum)]
    #[allow(clippy::enum_variant_names)]
    pub enum RangeBound {
        /// postgres '[]'
        LowerBoundInclusiveUpperBoundInclusive,
        /// postgres '[)'
        LowerBoundInclusiveUpperBoundExclusive,
        /// postgres '(]'
        LowerBoundExclusiveUpperBoundInclusive,
        /// postgres '()'
        LowerBoundExclusiveUpperBoundExclusive,
    }

    /// The [`Record`] (a.k.a. tuple) SQL type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - Any tuple which can be serialized to each of the elements
    ///   (note: There are major caveats, see the section below)
    ///
    /// ### [`FromSql`] impls
    ///
    /// - Any tuple which can be deserialized from each of the elements.
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    ///
    /// ### Caveats about serialization
    ///
    /// Typically in the documentation for SQL types, we use "`FromSql` impls"
    /// as a shorthand for "Rust types that you can use to represent this type".
    /// For every other type, that means there is specifically an implementation
    /// of the `FromSql` trait.
    ///
    /// However, PostgreSQL does not support transmission of anonymous record
    /// types as bind parameters. It only supports transmission for named
    /// composite types. For this reason, if you tried to do
    /// `int_tuple_col.eq((1, 2))`, we will generate the SQL `int_tuple_col =
    /// ($1, $2)` rather than `int_tuple_col = $1` as we would for anything
    /// else.
    ///
    /// This should not be visible during normal usage. The only time this would
    /// affect you is if you were attempting to use `sql_query` with tuples.
    /// Your code would not compile in that case, as the `ToSql` trait itself is
    /// not implemented.
    ///
    /// You can implement `ToSql` for named composite types. See [`WriteTuple`]
    /// for details.
    ///
    /// [`WriteTuple`]: super::super::super::serialize::WriteTuple
    /// [`Record`]: https://www.postgresql.org/docs/current/rowtypes.html
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 2249, array_oid = 2287))]
    pub struct Record<ST: 'static>(ST);

    /// Alias for [`SmallInt`](crate::sql_types::SmallInt)
    #[cfg(feature = "postgres_backend")]
    pub type SmallSerial = crate::sql_types::SmallInt;

    /// Alias for [`Integer`](crate::sql_types::Integer)
    #[cfg(feature = "postgres_backend")]
    pub type Serial = crate::sql_types::Integer;

    /// Alias for [`BigInt`](crate::sql_types::BigInt)
    #[cfg(feature = "postgres_backend")]
    pub type BigSerial = crate::sql_types::BigInt;

    /// The [`UUID`] SQL type. This type can only be used with `feature = "uuid"`
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`uuid::Uuid`][Uuid]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`uuid::Uuid`][Uuid]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [Uuid]: https://docs.rs/uuid/*/uuid/struct.Uuid.html
    /// [`UUID`]: https://www.postgresql.org/docs/current/datatype-uuid.html
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 2950, array_oid = 2951))]
    pub struct Uuid;

    /// Alias for `Binary`, to ensure `diesel print-schema` works
    pub type Bytea = crate::sql_types::Binary;

    #[doc(hidden)]
    pub type Bpchar = crate::sql_types::VarChar;

    /// The PostgreSQL [Money](https://www.postgresql.org/docs/current/static/datatype-money.html) type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`Cents` (also aliased as `PgMoney`)][PgMoney]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`Cents` (also aliased as `PgMoney`)][PgMoney]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [PgMoney]: crate::data_types::PgMoney
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// use diesel::data_types::Cents;
    ///
    /// table! {
    ///     items {
    ///         id -> Integer,
    ///         name -> VarChar,
    ///         price -> Money,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use diesel::insert_into;
    /// #     use self::items::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE items (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         price MONEY NOT NULL
    /// #     )").execute(connection).unwrap();
    /// let inserted_price = insert_into(items)
    ///     .values((name.eq("Shiny Thing"), price.eq(Cents(123_456))))
    ///     .returning(price)
    ///     .get_result(connection);
    /// assert_eq!(Ok(Cents(123_456)), inserted_price);
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 790, array_oid = 791))]
    pub struct Money;

    /// The [`MACADDR`](https://www.postgresql.org/docs/current/static/datatype-net-types.html) SQL type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - `[u8; 6]`
    ///
    /// ### [`FromSql`] impls
    ///
    /// - `[u8; 6]`
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// table! {
    ///     devices {
    ///         id -> Integer,
    ///         macaddr -> MacAddr,
    ///     }
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     use diesel::insert_into;
    /// #     use self::devices::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE devices (
    /// #         id SERIAL PRIMARY KEY,
    /// #         macaddr MACADDR NOT NULL
    /// #     )").execute(connection)?;
    /// let inserted_macaddr = insert_into(devices)
    ///     .values(macaddr.eq([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03]))
    ///     .returning(macaddr)
    ///     .get_result::<[u8; 6]>(connection)?;
    /// assert_eq!([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03], inserted_macaddr);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 829, array_oid = 1040))]
    pub struct MacAddr;

    /// Alias for `MacAddr` to be able to use it with `diesel print-schema`.
    pub type Macaddr = MacAddr;

    /// The [`MACADDR8`](https://www.postgresql.org/docs/current/static/datatype-net-types.html) SQL type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - `[u8; 8]`
    ///
    /// ### [`FromSql`] impls
    ///
    /// - `[u8; 8]`
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// table! {
    ///     devices {
    ///         id -> Integer,
    ///         macaddr -> MacAddr8,
    ///     }
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     use diesel::insert_into;
    /// #     use self::devices::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE devices (
    /// #         id SERIAL PRIMARY KEY,
    /// #         macaddr MACADDR8 NOT NULL
    /// #     )").execute(connection)?;
    /// let inserted_macaddr = insert_into(devices)
    ///     .values(macaddr.eq([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03, 0x04, 0x05]))
    ///     .returning(macaddr)
    ///     .get_result::<[u8; 8]>(connection)?;
    /// assert_eq!([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03, 0x04, 0x05], inserted_macaddr);
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 774, array_oid = 775))]
    pub struct MacAddr8;

    /// Alias for `MacAddr` to be able to use it with `diesel print-schema`.
    pub type Macaddr8 = MacAddr8;

    /// The [`INET`](https://www.postgresql.org/docs/current/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"` or `feature = "ipnet-address"`.
    ///
    /// ### [`ToSql`] impls
    ///
    #[cfg_attr(
        feature = "network-address",
        doc = " - [`ipnetwork::IpNetwork`][IpNetwork]"
    )]
    #[cfg_attr(feature = "ipnet-address", doc = " - [`ipnet::IpNet`][IpNet]")]
    #[cfg_attr(
        not(any(feature = "network-address", feature = "ipnet-address")),
        doc = "N/A"
    )]
    ///
    /// ### [`FromSql`] impls
    ///
    #[cfg_attr(
        feature = "network-address",
        doc = " - [`ipnetwork::IpNetwork`][IpNetwork]"
    )]
    #[cfg_attr(feature = "ipnet-address", doc = " - [`ipnet::IpNet`][IpNet]")]
    #[cfg_attr(
        not(any(feature = "network-address", feature = "ipnet-address")),
        doc = "N/A"
    )]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    #[cfg_attr(
        feature = "network-address",
        doc = " [IpNetwork]: ipnetwork::IpNetwork"
    )]
    #[cfg_attr(feature = "ipnet-address", doc = " [IpNet]: ipnet::IpNet")]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// table! {
    ///     clients {
    ///         id -> Integer,
    ///         ip_address -> Inet,
    ///     }
    /// }
    ///
    /// # #[cfg(any(feature = "network-address", feature = "ipnet-address"))]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// #     use diesel::insert_into;
    /// #     use self::clients::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address INET NOT NULL
    /// #     )").execute(connection)?;
    /// // Parsing "ipnet::IpNet" would also work.
    /// let addr = "10.1.9.32/32".parse::<ipnetwork::IpNetwork>()?;
    /// let inserted_address = insert_into(clients)
    ///     .values(ip_address.eq(&addr))
    ///     .returning(ip_address)
    ///     .get_result(connection)?;
    /// assert_eq!(addr, inserted_address);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(any(feature = "network-address", feature = "ipnet-address")))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 869, array_oid = 1041))]
    pub struct Inet;

    /// The [`CIDR`](https://www.postgresql.org/docs/postgresql/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"` or `feature = "ipnet-address"`.
    ///
    /// ### [`ToSql`] impls
    ///
    #[cfg_attr(
        feature = "network-address",
        doc = " - [`ipnetwork::IpNetwork`][IpNetwork]"
    )]
    #[cfg_attr(feature = "ipnet-address", doc = " - [`ipnet::IpNet`][IpNet]")]
    #[cfg_attr(
        not(any(feature = "network-address", feature = "ipnet-address")),
        doc = "N/A"
    )]
    ///
    /// ### [`FromSql`] impls
    ///
    #[cfg_attr(
        feature = "network-address",
        doc = " - [`ipnetwork::IpNetwork`][IpNetwork]"
    )]
    #[cfg_attr(feature = "ipnet-address", doc = " - [`ipnet::IpNet`][IpNet]")]
    #[cfg_attr(
        not(any(feature = "network-address", feature = "ipnet-address")),
        doc = "N/A"
    )]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    #[cfg_attr(
        feature = "network-address",
        doc = " [IpNetwork]: ipnetwork::IpNetwork"
    )]
    #[cfg_attr(feature = "ipnet-address", doc = " [IpNet]: ipnet::IpNet")]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// # include!("../../doctest_setup.rs");
    /// table! {
    ///     clients {
    ///         id -> Integer,
    ///         ip_address -> Cidr,
    ///     }
    /// }
    ///
    /// # #[cfg(any(feature = "network-address", feature = "ipnet-address"))]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// #     use diesel::insert_into;
    /// #     use self::clients::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address CIDR NOT NULL
    /// #     )").execute(connection)?;
    /// // Parsing "ipnet::IpNet" would also work.
    /// let addr = "10.1.9.32/32".parse::<ipnetwork::IpNetwork>()?;
    /// let inserted_addr = insert_into(clients)
    ///     .values(ip_address.eq(&addr))
    ///     .returning(ip_address)
    ///     .get_result(connection)?;
    /// assert_eq!(addr, inserted_addr);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(any(feature = "network-address", feature = "ipnet-address")))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 650, array_oid = 651))]
    pub struct Cidr;

    /// The [`"char"`] SQL type. This is a PostgreSQL specific type. Used for e.g. [setweight]. [Do not use in user tables].
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`u8`]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`u8`]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [`u8`]: https://doc.rust-lang.org/nightly/std/primitive.u8.html
    /// [`"char"`]: https://www.postgresql.org/docs/current/datatype-character.html#DATATYPE-CHARACTER-SPECIAL-TABLE
    /// [setweight]: https://www.postgresql.org/docs/current/functions-textsearch.html
    /// [Do not use in user tables]: https://www.postgresql.org/docs/current/datatype-character.html#DATATYPE-CHARACTER-SPECIAL-TABLE
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 18, array_oid = 1002))]
    pub struct CChar;

    /// The [`Citext`] SQL type. This is a PostgreSQL specific type.
    ///
    /// Strings must be valid UTF-8.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`String`]
    /// - [`&str`][str]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`String`]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [`Citext`]: https://www.postgresql.org/docs/current/citext.html
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(name = "citext"))]
    pub struct Citext;

    /// The [`Jsonpath`] SQL type. This is a PostgreSQL specific type.
    #[cfg(feature = "postgres_backend")]
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[diesel(postgres_type(oid = 4072, array_oid = 4073))]
    pub struct Jsonpath;
}

mod ops {
    use super::sql_types::*;
    use crate::sql_types::ops::*;
    use crate::sql_types::{Bigint, Interval};

    impl Add for Timestamptz {
        type Rhs = Interval;
        type Output = Timestamptz;
    }

    impl Sub for Timestamptz {
        type Rhs = Interval;
        type Output = Timestamptz;
    }

    impl Add for Cidr {
        type Rhs = Bigint;
        type Output = Inet;
    }

    impl Add for Inet {
        type Rhs = Bigint;
        type Output = Inet;
    }

    impl Sub for Cidr {
        type Rhs = Bigint;
        type Output = Inet;
    }

    impl Sub for Inet {
        type Rhs = Bigint;
        type Output = Inet;
    }
}
