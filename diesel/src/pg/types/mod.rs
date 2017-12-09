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

/// PostgreSQL specific SQL types
///
/// Note: All types in this module can be accessed through `diesel::types`
pub mod sql_types {
    /// The `OID` SQL type. This is a PostgreSQL specific type.
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`u32`][u32]
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`u32`][u32]
    ///
    /// [u32]: https://doc.rust-lang.org/nightly/std/primitive.u32.html
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Oid;

    /// The "timestamp with time zone" SQL type, which PostgreSQL abbreviates
    /// to `timestamptz`.
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`PgTimestamp`][PgTimestamp]
    /// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
    /// - [`chrono::DateTime`][DateTime] with `feature = "chrono"`
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`PgTimestamp`][PgTimestamp]
    /// - [`chrono::NaiveDateTime`][NaiveDateTime] with `feature = "chrono"`
    /// - [`chrono::DateTime`][DateTime] with `feature = "chrono"`
    ///
    /// [PgTimestamp]: /diesel/pg/data_types/struct.PgTimestamp.html
    /// [NaiveDateTime]: https://lifthrasiir.github.io/rust-chrono/chrono/naive/datetime/struct.NaiveDateTime.html
    /// [DateTime]: https://lifthrasiir.github.io/rust-chrono/chrono/datetime/struct.DateTime.html
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Timestamptz;

    /// The `Array` SQL type. This wraps another type to represent a SQL array of
    /// that type. Multidimensional arrays are not supported, nor are arrays
    /// containing null.
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`Vec<T>`][Vec] for any `T` which implements `ToSql<ST>`
    /// - [`&[T]`][slice] for any `T` which implements `ToSql<ST>`
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`Vec<T>`][Vec] for any `T` which implements `ToSql<ST>`
    ///
    /// [Vec]: https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html
    /// [slice]: https://doc.rust-lang.org/nightly/std/primitive.slice.html
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Array<ST>(ST);

    /// The `Range` SQL type. This wraps another type to represent a SQL range of
    /// that type.
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`(Bound<T>, Bound<T>)`][bound] for any `T` which implements `ToSql<ST>`.
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`(Bound<T>, Bound<T>)`][bound] for any `T` which implements `FromSql<ST>`.
    ///
    /// [bound]: https://doc.rust-lang.org/std/collections/enum.Bound.html
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Range<ST>(ST);

    #[doc(hidden)]
    pub type Int4range = Range<::types::Int4>;
    #[doc(hidden)]
    pub type Int8range = Range<::types::Int8>;
    #[doc(hidden)]
    pub type Daterange = Range<::types::Date>;
    #[doc(hidden)]
    pub type Numrange = Range<::types::Numeric>;
    #[doc(hidden)]
    pub type Tsrange = Range<::types::Timestamp>;
    #[doc(hidden)]
    pub type Tstzrange = Range<::types::Timestamptz>;

    /// Alias for `SmallInt`
    pub type SmallSerial = ::types::SmallInt;

    /// Alias for `Integer`
    pub type Serial = ::types::Integer;

    /// Alias for `BigInt`
    pub type BigSerial = ::types::BigInt;

    #[cfg(feature = "uuid")]
    /// The `UUID` SQL type. This type can only be used with `feature = "uuid"`
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`uuid::Uuid`][Uuid]
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`uuid::Uuid`][Uuid]
    ///
    /// [Uuid]: https://doc.rust-lang.org/uuid/uuid/struct.Uuid.html
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Uuid;

    /// Alias for `Binary`, to ensure `infer_schema!` works
    pub type Bytea = ::types::Binary;

    #[doc(hidden)]
    pub type Bpchar = ::types::VarChar;

    #[cfg(feature = "serde_json")]
    /// The JSON SQL type.  This type can only be used with `feature =
    /// "serde_json"`
    ///
    /// Normally you should prefer [`Jsonb`](struct.Jsonb.html) instead, for the reasons
    /// discussed there.
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`serde_json::Value`][Value]
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`serde_json`][Value]
    ///
    /// [Value]: https://docs.serde.rs/serde_json/value/enum.Value.html
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Json;

    #[cfg(feature = "serde_json")]
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
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`serde_json::Value`][Value]
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`serde_json`][Value]
    ///
    /// [Value]: https://docs.serde.rs/serde_json/value/enum.Value.html
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
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Jsonb;

    /// The PostgreSQL [Money](https://www.postgresql.org/docs/9.1/static/datatype-money.html) type.
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`Cents` (also aliased as `PgMoney`)][PgMoney]
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`Cents` (also aliased as `PgMoney`)][PgMoney]
    ///
    /// [PgMoney]: /diesel/pg/data_types/struct.PgMoney.html
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
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Money;

    #[cfg(feature = "network-address")]
    /// The [`MACADDR`](https://www.postgresql.org/docs/9.6/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"`
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - `[u8; 6]`
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - `[u8; 6]`
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
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MacAddr;

    #[cfg(feature = "network-address")]
    #[doc(hidden)]
    /// Alias for `MacAddr` to be able to use it with `infer_schema`.
    pub type Macaddr = MacAddr;

    #[cfg(feature = "network-address")]
    /// The [`INET`](https://www.postgresql.org/docs/9.6/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"`
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// [IpNetwork]: https://docs.rs/ipnetwork/0.12.2/ipnetwork/enum.IpNetwork.html
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
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Inet;

    #[cfg(feature = "network-address")]
    /// The [`CIDR`](https://www.postgresql.org/docs/9.6/static/datatype-net-types.html) SQL type. This type can only be used with `feature = "network-address"`
    ///
    /// ### [`ToSql`](/diesel/types/trait.ToSql.html) impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// ### [`FromSql`](/diesel/types/trait.FromSql.html) impls
    ///
    /// - [`ipnetwork::IpNetwork`][IpNetwork]
    ///
    /// [IpNetwork]: https://docs.rs/ipnetwork/0.12.2/ipnetwork/enum.IpNetwork.html
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
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Cidr;
}
