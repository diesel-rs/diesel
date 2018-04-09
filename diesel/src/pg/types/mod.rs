//! PostgreSQL specific types

mod array;
mod ranges;
#[doc(hidden)]
pub mod date_and_time;
#[doc(hidden)]
pub mod floats;
#[cfg(feature = "network-address")]
mod network_address;
mod integers;
mod numeric;
mod primitives;
#[cfg(feature = "uuid")]
mod uuid;
#[cfg(feature = "serde_json")]
mod json;
#[doc(hidden)]
pub mod money;
pub mod geometric;

/// PostgreSQL specific SQL types
///
/// Note: All types in this module can be accessed through `diesel::sql_types`
pub mod sql_types {
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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [`PgTimestamp`]: ../../data_types/struct.PgTimestamp.html
    /// [`chrono::NaiveDateTime`]: ../../../../chrono/naive/struct.NaiveDateTime.html
    /// [`chrono::DateTime`]: ../../../../chrono/struct.DateTime.html
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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [Vec]: https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html
    /// [slice]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
    #[derive(Debug, Clone, Copy, Default, QueryId)]
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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [bound]: https://doc.rust-lang.org/std/collections/enum.Bound.html
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    pub struct Range<ST>(ST);

    #[doc(hidden)]
    pub type Int4range = Range<::sql_types::Int4>;
    #[doc(hidden)]
    pub type Int8range = Range<::sql_types::Int8>;
    #[doc(hidden)]
    pub type Daterange = Range<::sql_types::Date>;
    #[doc(hidden)]
    pub type Numrange = Range<::sql_types::Numeric>;
    #[doc(hidden)]
    pub type Tsrange = Range<::sql_types::Timestamp>;
    #[doc(hidden)]
    pub type Tstzrange = Range<::sql_types::Timestamptz>;

    /// Alias for `SmallInt`
    pub type SmallSerial = ::sql_types::SmallInt;

    /// Alias for `Integer`
    pub type Serial = ::sql_types::Integer;

    /// Alias for `BigInt`
    pub type BigSerial = ::sql_types::BigInt;

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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [Uuid]: https://doc.rust-lang.org/uuid/uuid/struct.Uuid.html
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "2950", array_oid = "2951")]
    pub struct Uuid;

    /// Alias for `Binary`, to ensure `infer_schema!` works
    #[doc(hidden)]
    pub type Bytea = ::sql_types::Binary;

    #[doc(hidden)]
    pub type Bpchar = ::sql_types::VarChar;

    /// The JSON SQL type.  This type can only be used with `feature =
    /// "serde_json"`
    ///
    /// Normally you should prefer [`Jsonb`](struct.Jsonb.html) instead, for the reasons
    /// discussed there.
    ///
    /// ### [`ToSql`] impls
    ///
    /// - [`serde_json::Value`]
    ///
    /// ### [`FromSql`] impls
    ///
    /// - [`serde_json::Value`]
    ///
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [`serde_json::Value`]: ../../../../serde_json/value/enum.Value.html
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "114", array_oid = "199")]
    pub struct Json;

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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [`serde_json::Value`]: ../../../../serde_json/value/enum.Value.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// extern crate serde_json;
    /// # #[macro_use] extern crate diesel;
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
    /// # fn main() {
    /// #     use diesel::insert_into;
    /// #     use contacts::dsl::*;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").unwrap();
    /// let santas_address: serde_json::Value = serde_json::from_str(r#"{
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// }"#).unwrap();
    /// let inserted_address = insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .returning(address)
    ///     .get_result(&connection);
    /// assert_eq!(Ok(santas_address), inserted_address);
    /// # }
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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [PgMoney]: ../../data_types/struct.PgMoney.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// # #[macro_use] extern crate diesel;
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
    /// #     use items::dsl::*;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE items (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         price MONEY NOT NULL
    /// #     )").unwrap();
    /// let inserted_price = insert_into(items)
    ///     .values((name.eq("Shiny Thing"), price.eq(Cents(123_456))))
    ///     .returning(price)
    ///     .get_result(&connection);
    /// assert_eq!(Ok(Cents(123_456)), inserted_price);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "790", array_oid = "791")]
    pub struct Money;

    /// The [`MACADDR`](https://www.postgresql.org/docs/9.6/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"`
    ///
    /// ### [`ToSql`] impls
    ///
    /// - `[u8; 6]`
    ///
    /// ### [`FromSql`] impls
    ///
    /// - `[u8; 6]`
    ///
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// table! {
    ///     devices {
    ///         id -> Integer,
    ///         macaddr -> MacAddr,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use diesel::insert_into;
    /// #     use devices::dsl::*;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE devices (
    /// #         id SERIAL PRIMARY KEY,
    /// #         macaddr MACADDR NOT NULL
    /// #     )").unwrap();
    /// let inserted_macaddr = insert_into(devices)
    ///     .values(macaddr.eq([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03]))
    ///     .returning(macaddr)
    ///     .get_result(&connection);
    /// assert_eq!(Ok([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03]), inserted_macaddr);
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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [IpNetwork]: ../../../../ipnetwork/enum.IpNetwork.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// extern crate ipnetwork;
    /// use ipnetwork::IpNetwork;
    ///
    /// table! {
    ///     clients {
    ///         id -> Integer,
    ///         ip_address -> Inet,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use diesel::insert_into;
    /// #     use clients::dsl::*;
    /// #     use std::str::FromStr;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address INET NOT NULL
    /// #     )").unwrap();
    /// let addr = IpNetwork::from_str("10.1.9.32/32").unwrap();
    /// let inserted_address = insert_into(clients)
    ///     .values(ip_address.eq(&addr))
    ///     .returning(ip_address)
    ///     .get_result(&connection);
    /// assert_eq!(Ok(addr), inserted_address);
    /// # }
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
    /// [`ToSql`]: ../../../serialize/trait.ToSql.html
    /// [`FromSql`]: ../../../deserialize/trait.FromSql.html
    /// [IpNetwork]: ../../../../ipnetwork/enum.IpNetwork.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// extern crate ipnetwork;
    /// use ipnetwork::IpNetwork;
    ///
    /// table! {
    ///     clients {
    ///         id -> Integer,
    ///         ip_address -> Cidr,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use diesel::insert_into;
    /// #     use clients::dsl::*;
    /// #     use std::str::FromStr;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address CIDR NOT NULL
    /// #     )").unwrap();
    /// let addr = IpNetwork::from_str("10.1.9.32/32").unwrap();
    /// let inserted_addr = insert_into(clients)
    ///     .values(ip_address.eq(&addr))
    ///     .returning(ip_address)
    ///     .get_result(&connection);
    /// assert_eq!(Ok(addr), inserted_addr);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "650", array_oid = "651")]
    pub struct Cidr;

    /// The PostgreSQL [Point](https://www.postgresql.org/docs/9.3/static/datatype-geometric.html) type.
    ///
    /// ### [`ToSql`](::serialize::ToSql) impls
    ///
    /// - [`PgPoint`](::pg::data_types::PgPoint)
    ///
    /// ### [`FromSql`](::deserialize::FromSql) impls
    ///
    /// - [`PgPoint`](::pg::data_types::PgPoint)
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![allow(dead_code)]
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// use diesel::data_types::PgPoint;
    ///
    /// table! {
    ///     items {
    ///         id -> Integer,
    ///         name -> VarChar,
    ///         location -> Point,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use diesel::insert_into;
    /// #     use items::dsl::*;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE items (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         location POINT NOT NULL
    /// #     )").unwrap();
    /// let inserted_location = insert_into(items)
    ///     .values((name.eq("Shiny Thing"), location.eq(PgPoint(3.1, 9.4))))
    ///     .returning(location)
    ///     .get_result(&connection);
    /// assert_eq!(Ok(PgPoint(3.1, 9.4)), inserted_location);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default, QueryId, SqlType)]
    #[postgres(oid = "600", array_oid = "1017")]
    pub struct Point;
}
