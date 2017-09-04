mod array;
mod ranges;
pub mod date_and_time;
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
    #[derive(Debug, Clone, Copy, Default)] pub struct Oid;

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
    #[derive(Debug, Clone, Copy, Default)] pub struct Timestamptz;

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
    #[derive(Debug, Clone, Copy, Default)] pub struct Array<ST>(ST);

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
    #[derive(Debug, Clone, Copy, Default)] pub struct Range<ST>(ST);

    #[doc(hidden)] pub type Int4range = Range<::types::Int4>;
    #[doc(hidden)] pub type Int8range = Range<::types::Int8>;
    #[doc(hidden)] pub type Daterange = Range<::types::Date>;
    #[doc(hidden)] pub type Numrange = Range<::types::Numeric>;
    #[doc(hidden)] pub type Tsrange = Range<::types::Timestamp>;
    #[doc(hidden)] pub type Tstzrange = Range<::types::Timestamptz>;

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
    #[derive(Debug, Clone, Copy, Default)] pub struct Uuid;

    /// Alias for `Binary`, to ensure `infer_schema!` works
    pub type Bytea = ::types::Binary;

    #[doc(hidden)]
    pub type Bpchar = ::types::VarChar;

    #[doc(hidden)]
    pub type Citext = ::types::Text;

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
    #[derive(Debug, Clone, Copy, Default)] pub struct Json;

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
    /// # #[macro_use] extern crate diesel_codegen;
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// #[derive(Queryable)]
    /// struct Contact {
    ///     id: i32,
    ///     name: String,
    ///     address: serde_json::Value,
    /// }
    ///
    /// #[derive(Insertable)]
    /// #[table_name="contacts"]
    /// struct NewContact {
    ///     name: String,
    ///     address: serde_json::Value,
    /// }
    ///
    /// table! {
    ///     contacts {
    ///         id -> Integer,
    ///         name -> VarChar,
    ///         address -> Jsonb,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use self::diesel::insert;
    /// #     use self::contacts::dsl::*;
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
    /// let new_contact = NewContact {
    ///     name: "Claus".into(),
    ///     address: santas_address.clone()
    /// };
    /// let inserted_contact = insert(&new_contact).into(contacts)
    ///     .get_result::<Contact>(&connection).unwrap();
    /// assert_eq!(santas_address, inserted_contact.address);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default)] pub struct Jsonb;

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
    /// # #[macro_use] extern crate diesel_codegen;
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    ///
    /// use diesel::data_types::Cents;
    ///
    /// #[derive(Queryable)]
    /// struct Item {
    ///     id: i32,
    ///     name: String,
    ///     price: Cents,
    /// }
    ///
    /// #[derive(Insertable)]
    /// #[table_name="items"]
    /// struct NewItem {
    ///     name: String,
    ///     price: Cents,
    /// }
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
    /// #     use self::diesel::insert;
    /// #     use self::items::dsl::*;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE items (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         price MONEY NOT NULL
    /// #     )").unwrap();
    /// let new_item = NewItem {
    ///     name: "Shiny Thing".into(),
    ///     price: Cents(123_456),
    /// };
    /// let inserted_item = insert(&new_item).into(items)
    ///     .get_result::<Item>(&connection).unwrap();
    /// assert_eq!(Cents(123_456), inserted_item.price);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default)] pub struct Money;

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
    /// # #[macro_use] extern crate diesel_codegen;
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// # use diesel::types::MacAddr;
    ///
    /// #[derive(Queryable)]
    /// struct Device {
    ///     id: i32,
    ///     macaddr: [u8; 6],
    /// }
    ///
    /// #[derive(Insertable)]
    /// #[table_name="devices"]
    /// struct NewDevice {
    ///     macaddr: [u8;6],
    /// }
    ///
    /// table! {
    ///     devices {
    ///         id -> Integer,
    ///         macaddr -> MacAddr,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use self::diesel::insert;
    /// #     use self::devices::dsl::*;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE devices (
    /// #         id SERIAL PRIMARY KEY,
    /// #         macaddr MACADDR NOT NULL
    /// #     )").unwrap();
    /// let new_device = NewDevice {
    ///     macaddr: [0x08, 0x00, 0x2b, 0x01, 0x02, 0x03],
    /// };
    /// let inserted_device = insert(&new_device).into(devices)
    ///     .get_result::<Device>(&connection).unwrap();
    /// assert_eq!([0x08, 0x00, 0x2b, 0x01, 0x02, 0x03], inserted_device.macaddr);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default)] pub struct MacAddr;

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
    /// # #[macro_use] extern crate diesel_codegen;
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// extern crate ipnetwork;
    /// # use diesel::types::Inet;
    /// use ipnetwork::IpNetwork;
    ///
    /// #[derive(Queryable)]
    /// struct Client {
    ///     id: i32,
    ///     ip_address: IpNetwork,
    /// }
    ///
    /// #[derive(Insertable)]
    /// #[table_name="clients"]
    /// struct NewClient {
    ///     ip_address: IpNetwork,
    /// }
    ///
    /// table! {
    ///     clients {
    ///         id -> Integer,
    ///         ip_address -> Inet,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use self::diesel::insert;
    /// #     use self::clients::dsl::*;
    /// #     use std::str::FromStr;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address INET NOT NULL
    /// #     )").unwrap();
    /// let new_client = NewClient {
    ///     ip_address: "10.1.9.32/32".parse().unwrap(),
    /// };
    /// let inserted_client = insert(&new_client).into(clients)
    ///     .get_result::<Client>(&connection).unwrap();
    /// assert_eq!(IpNetwork::from_str("10.1.9.32/32").unwrap(), inserted_client.ip_address);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default)] pub struct Inet;

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
    /// # #[macro_use] extern crate diesel_codegen;
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #     users {
    /// #         id -> Serial,
    /// #         name -> VarChar,
    /// #     }
    /// # }
    /// #
    /// extern crate ipnetwork;
    /// # use diesel::types::Cidr;
    /// use ipnetwork::IpNetwork;
    ///
    /// #[derive(Queryable)]
    /// struct Client {
    ///     id: i32,
    ///     ip_address: IpNetwork,
    /// }
    ///
    /// #[derive(Insertable)]
    /// #[table_name="clients"]
    /// struct NewClient {
    ///     ip_address: IpNetwork,
    /// }
    ///
    /// table! {
    ///     clients {
    ///         id -> Integer,
    ///         ip_address -> Cidr,
    ///     }
    /// }
    ///
    /// # fn main() {
    /// #     use self::diesel::insert;
    /// #     use self::clients::dsl::*;
    /// #     use std::str::FromStr;
    /// #     let connection = connection_no_data();
    /// #     connection.execute("CREATE TABLE clients (
    /// #         id SERIAL PRIMARY KEY,
    /// #         ip_address CIDR NOT NULL
    /// #     )").unwrap();
    /// let new_client = NewClient {
    ///     ip_address: "10.1.9.32/32".parse().unwrap(),
    /// };
    /// let inserted_client = insert(&new_client).into(clients)
    ///     .get_result::<Client>(&connection).unwrap();
    /// assert_eq!(IpNetwork::from_str("10.1.9.32/32").unwrap(), inserted_client.ip_address);
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, Default)] pub struct Cidr;
}
