//! PostgreSQL specific types

mod array;
#[doc(hidden)]
pub mod date_and_time;
#[doc(hidden)]
pub mod floats;
mod integers;
#[cfg(feature = "serde_json")]
mod json;
mod mac_addr;
#[doc(hidden)]
pub mod money;
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

    /// The `OID` SQL type. This is a PostgreSQL specific type.
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
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "26", array_oid = "1018")]
    pub struct Oid;

    /// The "timestamp with time zone" SQL type, which PostgreSQL abbreviates
    /// to `timestamptz`.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`PgTimestamp`]
    /// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
    /// - [`chrono::DateTime`] with `feature = "chrono"`
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`PgTimestamp`]
    /// - [`chrono::NaiveDateTime`] with `feature = "chrono"`
    /// - [`chrono::DateTime`] with `feature = "chrono"`
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [`PgTimestamp`]: super::super::data_types::PgTimestamp
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
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "1184", array_oid = "1185")]
    pub struct Timestamptz;

    /// The `Array` SQL type.
    ///
    /// This wraps another type to represent a SQL array of that type.
    /// Multidimensional arrays are not supported,
    /// nor are arrays containing null.
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
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    pub struct Array<ST>(ST);

    /// The `Range` SQL type.
    ///
    /// This wraps another type to represent a SQL range of that type.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`(Bound<T>, Bound<T>)`][bound] for any `T` which implements `ToSql<ST>`.
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`(Bound<T>, Bound<T>)`][bound] for any `T` which implements `FromSql<ST>`.
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    /// [bound]: std::collections::Bound
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    pub struct Range<ST>(ST);

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

    /// The `Record` (a.k.a. tuple) SQL type.
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
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "2249", array_oid = "2287")]
    pub struct Record<ST>(ST);

    /// Alias for `SmallInt`
    pub type SmallSerial = crate::sql_types::SmallInt;

    /// Alias for `Integer`
    pub type Serial = crate::sql_types::Integer;

    /// Alias for `BigInt`
    pub type BigSerial = crate::sql_types::BigInt;

    /// The `UUID` SQL type. This type can only be used with `feature = "uuid"`
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
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "2950", array_oid = "2951")]
    pub struct Uuid;

    /// Alias for `Binary`, to ensure `infer_schema!` works
    #[doc(hidden)]
    pub type Bytea = crate::sql_types::Binary;

    #[doc(hidden)]
    pub type Bpchar = crate::sql_types::VarChar;

    /// The `jsonb` SQL type.  This type can only be used with `feature =
    /// "serde_json"`
    ///
    /// `jsonb` offers [several advantages][adv] over regular JSON:
    ///
    /// > There are two JSON data types: `json` and `jsonb`. They accept almost
    /// > identical sets of values as input. The major practical difference
    /// > is one of efficiency. The `json` data type stores an exact copy of
    /// > the input text, which processing functions must reparse on each
    /// > execution; while `jsonb` data is stored in a decomposed binary format
    /// > that makes it slightly slower to input due to added conversion
    /// > overhead, but significantly faster to process, since no reparsing
    /// > is needed. `jsonb` also supports indexing, which can be a significant
    /// > advantage.
    /// >
    /// > ...In general, most applications should prefer to store JSON data as
    /// > `jsonb`, unless there are quite specialized needs, such as legacy
    /// > assumptions about ordering of object keys.
    ///
    /// [adv]: https://www.postgresql.org/docs/9.6/static/datatype-json.html
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`serde_json::Value`]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`serde_json::Value`]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    #[cfg_attr(
        feature = "serde_json",
        doc = "[`serde_json::Value`]: serde_json::value::Value"
    )]
    #[cfg_attr(
        not(feature = "serde_json"),
        doc = "[`serde_json::Value`]: https://docs.rs/serde_json/1.0.64/serde_json/value/enum.Value.html"
    )]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// # include!("../../doctest_setup.rs");
    /// #
    /// table! {
    ///     contacts {
    ///         id -> Integer,
    ///         name -> VarChar,
    ///         address -> Jsonb,
    ///     }
    /// }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     use diesel::insert_into;
    /// #     use self::contacts::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(connection)?;
    /// let santas_address: serde_json::Value = serde_json::from_str(r#"{
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// }"#)?;
    /// let inserted_address = insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .returning(address)
    ///     .get_result::<serde_json::Value>(connection)?;
    /// assert_eq!(santas_address, inserted_address);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn main() {}
    /// ```
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "3802", array_oid = "3807")]
    pub struct Jsonb;

    /// The PostgreSQL [Money](https://www.postgresql.org/docs/9.1/static/datatype-money.html) type.
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
    /// #     connection.execute("CREATE TABLE items (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         price MONEY NOT NULL
    /// #     )").unwrap();
    /// let inserted_price = insert_into(items)
    ///     .values((name.eq("Shiny Thing"), price.eq(Cents(123_456))))
    ///     .returning(price)
    ///     .get_result(connection);
    /// assert_eq!(Ok(Cents(123_456)), inserted_price);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "790", array_oid = "791")]
    pub struct Money;

    /// The [`MACADDR`](https://www.postgresql.org/docs/9.6/static/datatype-net-types.html) SQL type.
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
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "829", array_oid = "1040")]
    pub struct MacAddr;

    #[doc(hidden)]
    /// Alias for `MacAddr` to be able to use it with `infer_schema`.
    pub type Macaddr = MacAddr;

    /// The [`INET`](https://www.postgresql.org/docs/9.6/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"`
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    #[cfg_attr(feature = "ipnetwork", doc = " [IpNetwork]: ipnetwork::IpNetwork")]
    #[cfg_attr(
        not(feature = "ipnetwork"),
        doc = " [IpNetwork]: https://docs.rs/ipnetwork/*/ipnetwork/enum.IpNetwork.html"
    )]
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
    /// # #[cfg(feature = "network-address")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use ipnetwork::IpNetwork;
    ///
    /// #     use diesel::insert_into;
    /// #     use self::clients::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address INET NOT NULL
    /// #     )").execute(connection)?;
    /// let addr = "10.1.9.32/32".parse::<IpNetwork>()?;
    /// let inserted_address = insert_into(clients)
    ///     .values(ip_address.eq(&addr))
    ///     .returning(ip_address)
    ///     .get_result(connection)?;
    /// assert_eq!(addr, inserted_address);
    /// #     Ok(())
    /// # }
    /// #
    /// # #[cfg(not(feature = "network-address"))]
    /// # fn main() {}
    /// ```
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "869", array_oid = "1041")]
    pub struct Inet;

    /// The [`CIDR`](https://www.postgresql.org/docs/9.6/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"`
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// [`ToSql`]: crate::serialize::ToSql
    /// [`FromSql`]: crate::deserialize::FromSql
    #[cfg_attr(feature = "ipnetwork", doc = " [IpNetwork]: ipnetwork::IpNetwork")]
    #[cfg_attr(
        not(feature = "ipnetwork"),
        doc = " [IpNetwork]: https://docs.rs/ipnetwork/*/ipnetwork/enum.IpNetwork.html"
    )]
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
    /// # #[cfg(feature = "network-address")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use ipnetwork::IpNetwork;
    ///
    /// #     use diesel::insert_into;
    /// #     use self::clients::dsl::*;
    /// #     let connection = &mut connection_no_data();
    /// #     diesel::sql_query("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address CIDR NOT NULL
    /// #     )").execute(connection)?;
    /// let addr = "10.1.9.32/32".parse::<IpNetwork>()?;
    /// let inserted_addr = insert_into(clients)
    ///     .values(ip_address.eq(&addr))
    ///     .returning(ip_address)
    ///     .get_result(connection)?;
    /// assert_eq!(addr, inserted_addr);
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "network-address"))]
    /// # fn main() {}
    /// ```
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "650", array_oid = "651")]
    pub struct Cidr;
}

mod ops {
    use super::sql_types::*;
    use crate::sql_types::ops::*;
    use crate::sql_types::{Bigint, Cidr, Inet, Interval};

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
