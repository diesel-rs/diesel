//! This module provides a container that allows to receive a dynamically
//! specified number of fields from the database.
//!
//!
//! ```rust
//! # mod connection_setup {
//! #     include!("../tests/connection_setup.rs");
//! # }
//! # use diesel::prelude::*;
//! # use diesel::sql_types::{Untyped};
//! # use diesel_dynamic_schema::{table, DynamicSelectClause};
//! # use diesel_dynamic_schema::dynamic_value::*;
//! # use diesel::dsl::sql_query;
//! # use diesel::deserialize::{self, FromSql};
//! #
//! # #[derive(PartialEq, Debug)]
//! # enum MyDynamicValue {
//! #     String(String),
//! #     Integer(i32),
//! # }
//! #
//! # #[cfg(feature = "postgres")]
//! # impl FromSql<Any, diesel::pg::Pg> for MyDynamicValue {
//! #     fn from_sql(value: diesel::pg::PgValue) -> deserialize::Result<Self> {
//! #         use diesel::pg::Pg;
//! #         use std::num::NonZeroU32;
//! #
//! #         const VARCHAR_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1043) };
//! #         const TEXT_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(25) };
//! #         const INTEGER_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(23) };
//! #
//! #         match value.get_oid() {
//! #             VARCHAR_OID | TEXT_OID => {
//! #                 <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(value)
//! #                     .map(MyDynamicValue::String)
//! #             }
//! #             INTEGER_OID => <i32 as FromSql<diesel::sql_types::Integer, Pg>>::from_sql(value)
//! #                 .map(MyDynamicValue::Integer),
//! #             e => Err(format!("Unknown type: {}", e).into()),
//! #         }
//! #     }
//! # }
//! #
//! # #[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
//! # impl FromSql<Any, diesel::sqlite::Sqlite> for MyDynamicValue {
//! #     fn from_sql(value: diesel::sqlite::SqliteValue) -> deserialize::Result<Self> {
//! #         use diesel::sqlite::{Sqlite, SqliteType};
//! #         match value.value_type() {
//! #             Some(SqliteType::Text) => {
//! #                 <String as FromSql<diesel::sql_types::Text, Sqlite>>::from_sql(value)
//! #                     .map(MyDynamicValue::String)
//! #             }
//! #             Some(SqliteType::Long) => {
//! #                 <i32 as FromSql<diesel::sql_types::Integer, Sqlite>>::from_sql(value)
//! #                     .map(MyDynamicValue::Integer)
//! #             }
//! #             _ => Err("Unknown data type".into()),
//! #         }
//! #     }
//! # }
//! #
//! # #[cfg(feature = "mysql")]
//! # impl FromSql<Any, diesel::mysql::Mysql> for MyDynamicValue {
//! #    fn from_sql(value: diesel::mysql::MysqlValue) -> deserialize::Result<Self> {
//! #         use diesel::mysql::{Mysql, MysqlType};
//! #         match value.value_type() {
//! #              MysqlType::String => {
//! #                  <String as FromSql<diesel::sql_types::Text, Mysql>>::from_sql(value)
//! #                      .map(MyDynamicValue::String)
//! #              }
//! #              MysqlType::Long => <i32 as FromSql<diesel::sql_types::Integer, Mysql>>::from_sql(value)
//! #                 .map(MyDynamicValue::Integer),
//! #             e => Err(format!("Unknown data type: {:?}", e).into()),
//! #         }
//! #     }
//! # }
//!
//! # #[cfg(feature = "mariadb")]
//! # impl FromSql<Any, diesel::mariadb::Mariadb> for MyDynamicValue {
//! #    fn from_sql(value: diesel::mariadb::MariadbValue) -> deserialize::Result<Self> {
//! #         use diesel::mariadb::{Mariadb, MariadbType};
//! #         match value.value_type() {
//! #              MariadbType::String => {
//! #                  <String as FromSql<diesel::sql_types::Text, Mariadb>>::from_sql(value)
//! #                      .map(MyDynamicValue::String)
//! #              }
//! #              MariadbType::Long => <i32 as FromSql<diesel::sql_types::Integer, Mariadb>>::from_sql(value)
//! #                 .map(MyDynamicValue::Integer),
//! #             e => Err(format!("Unknown data type: {:?}", e).into()),
//! #         }
//! #     }
//! # }
//! #
//! # fn result_main() -> QueryResult<()> {
//! #
//! # let conn = &mut connection_setup::establish_connection();
//! #
//! # // Create some example data by using typical SQL statements.
//! # connection_setup::create_user_table(conn);
//! # sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')").execute(conn)?;
//!
//!     let users = diesel_dynamic_schema::table("users");
//!     let id = users.column::<Untyped, _>("id");
//!     let name = users.column::<Untyped, _>("name");
//!
//!     let mut select = DynamicSelectClause::new();
//!
//!     select.add_field(id);
//!     select.add_field(name);
//!
//!     let actual_data: Vec<DynamicRow<NamedField<MyDynamicValue>>> =
//!         users.select(select).load(conn)?;
//!
//!     assert_eq!(
//!         actual_data[0]["name"],
//!         MyDynamicValue::String("Sean".into())
//!     );
//!     assert_eq!(
//!         actual_data[0][1],
//!         NamedField {
//!             name: "name".into(),
//!             value: MyDynamicValue::String("Sean".into())
//!         }
//!     );
//!
//! # Ok(())
//! # }
//! # result_main().unwrap()
//! ```
//!
//! It is required to provide your own inner type to hold the actual database value.
//!
//! ```rust
//! # use diesel_dynamic_schema::dynamic_value::Any;
//! # use diesel::deserialize::{self, FromSql};
//! #
//! #[derive(PartialEq, Debug)]
//! enum MyDynamicValue {
//!     String(String),
//!     Integer(i32),
//! }
//!
//! # #[cfg(feature = "postgres")]
//! impl FromSql<Any, diesel::pg::Pg> for MyDynamicValue {
//!     fn from_sql(value: diesel::pg::PgValue) -> deserialize::Result<Self> {
//!         use diesel::pg::Pg;
//!         use std::num::NonZeroU32;
//!
//!         const VARCHAR_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1043) };
//!         const TEXT_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(25) };
//!         const INTEGER_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(23) };
//!
//!         match value.get_oid() {
//!             VARCHAR_OID | TEXT_OID => {
//!                 <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(value)
//!                     .map(MyDynamicValue::String)
//!             }
//!             INTEGER_OID => <i32 as FromSql<diesel::sql_types::Integer, Pg>>::from_sql(value)
//!                 .map(MyDynamicValue::Integer),
//!             e => Err(format!("Unknown type: {}", e).into()),
//!         }
//!     }
//! }
//! ```

#[cfg(any(feature = "postgres", feature = "mysql", feature = "mariadb"))]
use crate::error::DynamicSchemaError;
use alloc::borrow::ToOwned;
#[cfg(any(feature = "postgres", feature = "mysql", feature = "mariadb"))]
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::iter::FromIterator;
use core::ops::{Index, IndexMut};
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::expression::TypedExpressionType;
use diesel::row::{Field, NamedRow, Row};
use diesel::QueryableByName;

/// A marker type used to indicate that
/// the provided `FromSql` impl does handle
/// any passed database value, independently
/// from the actual value kind
pub struct Any;

impl TypedExpressionType for Any {}

#[cfg(feature = "postgres")]
impl diesel::expression::QueryMetadata<Any> for diesel::pg::Pg {
    fn row_metadata(_lookup: &mut Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>) {
        out.push(None)
    }
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
impl diesel::expression::QueryMetadata<Any> for diesel::sqlite::Sqlite {
    fn row_metadata(_lookup: &mut Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>) {
        out.push(None)
    }
}

#[cfg(feature = "mysql")]
impl diesel::expression::QueryMetadata<Any> for diesel::mysql::Mysql {
    fn row_metadata(_lookup: &mut Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>) {
        out.push(None)
    }
}

#[cfg(feature = "mariadb")]
impl diesel::expression::QueryMetadata<Any> for diesel::mariadb::Mariadb {
    fn row_metadata(_lookup: &mut Self::MetadataLookup, out: &mut Vec<Option<Self::TypeMetadata>>) {
        out.push(None)
    }
}
/// A dynamically sized container that allows to receive
/// a not at compile time known number of columns from the database
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DynamicRow<I> {
    values: Vec<I>,
}

impl<I> From<DynamicRow<I>> for Vec<I> {
    fn from(row: DynamicRow<I>) -> Self {
        row.values
    }
}

impl<I> From<DynamicRow<NamedField<I>>> for Vec<I> {
    fn from(row: DynamicRow<NamedField<I>>) -> Self {
        row.values.into_iter().map(|f| f.value).collect()
    }
}

impl<I> From<Vec<I>> for DynamicRow<I> {
    fn from(values: Vec<I>) -> Self {
        Self { values }
    }
}

impl<I> AsRef<DynamicRow<I>> for DynamicRow<I> {
    fn as_ref(&self) -> &DynamicRow<I> {
        self
    }
}

/// A helper struct used as field type in `DynamicRow`
/// to also return the name of the field along with the
/// value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedField<I> {
    /// Name of the field
    pub name: String,
    /// Actual field value
    pub value: I,
}

impl<I> FromIterator<I> for DynamicRow<I> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        DynamicRow {
            values: iter.into_iter().collect(),
        }
    }
}

impl<I> DynamicRow<I> {
    /// Get the field value at the provided row index
    ///
    /// Returns `None` if the index is outside the bounds of the row
    pub fn get(&self, index: usize) -> Option<&I> {
        self.values.get(index)
    }

    /// Get the mutable field value at the provided row index
    ///
    /// Returns `None` if the index is outside the bounds of the row
    pub fn get_mut(&mut self, index: usize) -> Option<&mut I> {
        self.values.get_mut(index)
    }

    /// Get the number of fields in the current row
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the current row is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns an iterator over the values of the row
    pub fn iter(&self) -> impl Iterator<Item = &I> {
        self.values.iter()
    }

    /// Returns a mutable iterator over the values of the row
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut I> {
        self.values.iter_mut()
    }

    /// Create a new dynamic row from an existing database row
    ///
    /// This function is mostly useful for third party backends adding
    /// support for `diesel_dynamic_schema`
    pub fn from_row<'a, DB>(row: &impl Row<'a, DB>) -> deserialize::Result<Self>
    where
        DB: Backend,
        I: FromSql<Any, DB>,
    {
        let data = (0..row.field_count())
            .map(|i| {
                let field = Row::get(row, i).expect("We checked the field count above");

                I::from_nullable_sql(field.value())
            })
            .collect::<deserialize::Result<_>>()?;

        Ok(Self { values: data })
    }
}

impl<I> DynamicRow<NamedField<I>> {
    /// Get the field value by the provided field name
    ///
    /// Returns `None` if the field with the specified name is not found.
    /// If there are multiple fields with the same name, the behaviour
    /// of this function is unspecified.
    pub fn get_by_name<S: AsRef<str>>(&self, name: S) -> Option<&I> {
        self.values
            .iter()
            .find(|f| f.name == name.as_ref())
            .map(|f| &f.value)
    }

    /// Get the mutable field value by the provided field name
    ///
    /// Returns `None` if the field with the specified name is not found.
    /// If there are multiple fields with the same name, the behaviour
    /// of this function is unspecified.
    pub fn get_mut_by_name<S: AsRef<str>>(&mut self, name: S) -> Option<&mut I> {
        self.values
            .iter_mut()
            .find(|f| f.name == name.as_ref())
            .map(|f| &mut f.value)
    }
}

impl<I> DynamicRow<NamedField<Option<I>>> {
    /// Create a new dynamic row instance with corresponding field information from the given
    /// database row
    ///
    /// This function is mostly useful for third party backends adding
    /// support for `diesel_dynamic_schema`
    pub fn from_nullable_row<'a, DB>(row: &impl Row<'a, DB>) -> deserialize::Result<Self>
    where
        DB: Backend,
        I: FromSql<Any, DB>,
    {
        let data = (0..row.field_count())
            .map(|i| {
                let field = Row::get(row, i).expect("We checked the field count above");

                let value = match I::from_nullable_sql(field.value()) {
                    Ok(o) => Some(o),
                    Err(e) if e.is::<diesel::result::UnexpectedNullError>() => None,
                    Err(e) => return Err(e),
                };

                Ok(NamedField {
                    name: field
                        .field_name()
                        .ok_or("Try to load an unnamed field")?
                        .to_owned(),
                    value,
                })
            })
            .collect::<deserialize::Result<Vec<_>>>()?;
        Ok(DynamicRow { values: data })
    }
}

#[cfg(feature = "postgres")]
impl<I> QueryableByName<diesel::pg::Pg> for DynamicRow<I>
where
    I: FromSql<Any, diesel::pg::Pg>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::pg::Pg>) -> deserialize::Result<Self> {
        Self::from_row(row)
    }
}

#[cfg(feature = "mysql")]
impl<I> QueryableByName<diesel::mysql::Mysql> for DynamicRow<I>
where
    I: FromSql<Any, diesel::mysql::Mysql>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::mysql::Mysql>) -> deserialize::Result<Self> {
        Self::from_row(row)
    }
}

#[cfg(feature = "mariadb")]
impl<I> QueryableByName<diesel::mariadb::Mariadb> for DynamicRow<I>
where
    I: FromSql<Any, diesel::mariadb::Mariadb>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::mariadb::Mariadb>) -> deserialize::Result<Self> {
        Self::from_row(row)
    }
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
impl<I> QueryableByName<diesel::sqlite::Sqlite> for DynamicRow<I>
where
    I: FromSql<Any, diesel::sqlite::Sqlite>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::sqlite::Sqlite>) -> deserialize::Result<Self> {
        Self::from_row(row)
    }
}

impl<I, DB> QueryableByName<DB> for DynamicRow<Option<I>>
where
    DB: Backend,
    I: FromSql<Any, DB>,
{
    fn build<'a>(row: &impl NamedRow<'a, DB>) -> deserialize::Result<Self> {
        let data = (0..row.field_count())
            .map(|i| {
                let field = Row::get(row, i).expect("We checked the field count above");

                match I::from_nullable_sql(field.value()) {
                    Ok(o) => Ok(Some(o)),
                    Err(e) if e.is::<diesel::result::UnexpectedNullError>() => Ok(None),
                    Err(e) => Err(e),
                }
            })
            .collect::<deserialize::Result<_>>()?;

        Ok(Self { values: data })
    }
}

impl<I, DB> QueryableByName<DB> for DynamicRow<NamedField<I>>
where
    DB: Backend,
    I: FromSql<Any, DB>,
{
    fn build<'a>(row: &impl NamedRow<'a, DB>) -> deserialize::Result<Self> {
        let data = (0..row.field_count())
            .map(|i| {
                let field = Row::get(row, i).expect("We checked the field count above");

                let value = I::from_nullable_sql(field.value())?;

                Ok(NamedField {
                    name: field
                        .field_name()
                        .ok_or("Try to load an unnamed field")?
                        .to_owned(),
                    value,
                })
            })
            .collect::<deserialize::Result<Vec<_>>>()?;
        Ok(DynamicRow { values: data })
    }
}

#[cfg(feature = "postgres")]
impl<I> QueryableByName<diesel::pg::Pg> for DynamicRow<NamedField<Option<I>>>
where
    I: FromSql<Any, diesel::pg::Pg>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::pg::Pg>) -> deserialize::Result<Self> {
        Self::from_nullable_row(row)
    }
}

#[cfg(feature = "mysql")]
impl<I> QueryableByName<diesel::mysql::Mysql> for DynamicRow<NamedField<Option<I>>>
where
    I: FromSql<Any, diesel::mysql::Mysql>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::mysql::Mysql>) -> deserialize::Result<Self> {
        Self::from_nullable_row(row)
    }
}

#[cfg(feature = "mariadb")]
impl<I> QueryableByName<diesel::mariadb::Mariadb> for DynamicRow<NamedField<Option<I>>>
where
    I: FromSql<Any, diesel::mariadb::Mariadb>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::mariadb::Mariadb>) -> deserialize::Result<Self> {
        Self::from_nullable_row(row)
    }
}

#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
impl<I> QueryableByName<diesel::sqlite::Sqlite> for DynamicRow<NamedField<Option<I>>>
where
    I: FromSql<Any, diesel::sqlite::Sqlite>,
{
    fn build<'a>(row: &impl NamedRow<'a, diesel::sqlite::Sqlite>) -> deserialize::Result<Self> {
        Self::from_nullable_row(row)
    }
}

impl<I> Index<usize> for DynamicRow<I> {
    type Output = I;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl<I> IndexMut<usize> for DynamicRow<I> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.values[index]
    }
}

impl<'a, I> Index<&'a str> for DynamicRow<NamedField<I>> {
    type Output = I;

    fn index(&self, field_name: &'a str) -> &Self::Output {
        self.values
            .iter()
            .find(|f| f.name == field_name)
            .map(|f| &f.value)
            .expect("Field not found")
    }
}

impl<'a, I> IndexMut<&'a str> for DynamicRow<NamedField<I>> {
    fn index_mut(&mut self, field_name: &'a str) -> &mut Self::Output {
        self.values
            .iter_mut()
            .find(|f| f.name == field_name)
            .map(|f| &mut f.value)
            .expect("Field not found")
    }
}

impl<'a, I> Index<&'a String> for DynamicRow<NamedField<I>> {
    type Output = I;

    fn index(&self, field_name: &'a String) -> &Self::Output {
        self.index(field_name as &str)
    }
}

impl<'a, I> IndexMut<&'a String> for DynamicRow<NamedField<I>> {
    fn index_mut(&mut self, field_name: &'a String) -> &mut Self::Output {
        self.index_mut(field_name as &str)
    }
}

impl<I> Index<String> for DynamicRow<NamedField<I>> {
    type Output = I;

    fn index(&self, field_name: String) -> &Self::Output {
        self.index(&field_name)
    }
}

impl<I> IndexMut<String> for DynamicRow<NamedField<I>> {
    fn index_mut(&mut self, field_name: String) -> &mut Self::Output {
        self.index_mut(&field_name)
    }
}

impl<V> IntoIterator for DynamicRow<V> {
    type Item = V;
    type IntoIter = <Vec<V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a, V> IntoIterator for &'a DynamicRow<V> {
    type Item = &'a V;
    type IntoIter = <&'a Vec<V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl<'a, V> IntoIterator for &'a mut DynamicRow<V> {
    type Item = &'a mut V;
    type IntoIter = <&'a mut Vec<V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter_mut()
    }
}

/// A runtime database value decoded from [`Any`] without custom backend dispatch.
///
/// ```rust
/// # mod connection_setup {
/// #     include!("../tests/connection_setup.rs");
/// # }
/// # use diesel::prelude::*;
/// # use diesel::sql_query;
/// # use diesel::sql_types::Untyped;
/// # use diesel_dynamic_schema::{table, DynamicSelectClause};
/// # use diesel_dynamic_schema::dynamic_value::{DynamicRow, DynamicValue};
/// # fn run() -> QueryResult<()> {
/// # let conn = &mut connection_setup::establish_connection();
/// # connection_setup::create_user_table(conn);
/// # sql_query("INSERT INTO users (name) VALUES ('Sean')").execute(conn)?;
/// let users = table("users");
/// let name = users.column::<Untyped, _>("name");
///
/// let mut select = DynamicSelectClause::new();
/// select.add_field(name);
///
/// let rows: Vec<DynamicRow<DynamicValue>> = users.select(select).load(conn)?;
/// assert_eq!(rows[0][0], DynamicValue::Text("Sean".into()));
/// # Ok(())
/// # }
/// # run().unwrap();
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DynamicValue {
    /// SQL `NULL`. `Option<DynamicValue>` decodes it as `Some(Null)`.
    Null,
    /// A boolean, produced for PostgreSQL `BOOL`.
    Bool(bool),
    /// A signed integer, also used for SQLite, MySQL, and MariaDB booleans.
    Int(i64),
    /// An unsigned MySQL or MariaDB integer.
    UInt(u64),
    /// A floating-point number, widened to double precision.
    Float(f64),
    /// Text.
    Text(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
    /// A timestamp without a time zone.
    #[cfg(feature = "chrono")]
    Timestamp(chrono::NaiveDateTime),
    /// A UTC timestamp.
    #[cfg(feature = "chrono")]
    TimestampTz(chrono::DateTime<chrono::Utc>),
    /// A calendar date.
    #[cfg(feature = "chrono")]
    Date(chrono::NaiveDate),
    /// A PostgreSQL time of day.
    #[cfg(feature = "chrono")]
    Time(chrono::NaiveTime),
    /// A MySQL or MariaDB `TIME` duration.
    #[cfg(feature = "chrono")]
    Duration(chrono::Duration),
    /// An exact decimal.
    #[cfg(feature = "numeric")]
    Numeric(bigdecimal::BigDecimal),
    /// A UUID.
    #[cfg(feature = "uuid")]
    Uuid(uuid::Uuid),
    /// A JSON document.
    #[cfg(feature = "serde_json")]
    Json(serde_json::Value),
    /// A binary JSON document.
    #[cfg(feature = "serde_json")]
    Jsonb(serde_json::Value),
}

#[cfg(feature = "postgres")]
impl FromSql<Any, diesel::pg::Pg> for DynamicValue {
    fn from_sql(value: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        pg_dynamic_value(value)
    }

    fn from_nullable_sql(value: Option<diesel::pg::PgValue<'_>>) -> deserialize::Result<Self> {
        match value {
            Some(value) => pg_dynamic_value(value),
            None => Ok(DynamicValue::Null),
        }
    }
}

#[cfg(feature = "postgres")]
fn pg_dynamic_value(value: diesel::pg::PgValue<'_>) -> deserialize::Result<DynamicValue> {
    use diesel::pg::Pg;
    use diesel::sql_types as st;

    const BOOL: u32 = 16;
    const BYTEA: u32 = 17;
    const NAME: u32 = 19;
    const INT8: u32 = 20;
    const INT2: u32 = 21;
    const INT4: u32 = 23;
    const TEXT: u32 = 25;
    const JSON: u32 = 114;
    const FLOAT4: u32 = 700;
    const FLOAT8: u32 = 701;
    const BPCHAR: u32 = 1042;
    const VARCHAR: u32 = 1043;
    const DATE: u32 = 1082;
    const TIME: u32 = 1083;
    const TIMESTAMP: u32 = 1114;
    const TIMESTAMPTZ: u32 = 1184;
    const NUMERIC: u32 = 1700;
    const UUID: u32 = 2950;
    const JSONB: u32 = 3802;

    // Diesel's `FromSql` decodes each value. This function owns only dispatch.
    let oid = value.get_oid().get();
    Ok(match oid {
        BOOL => DynamicValue::Bool(<bool as FromSql<st::Bool, Pg>>::from_sql(value)?),
        INT2 => DynamicValue::Int(i64::from(<i16 as FromSql<st::SmallInt, Pg>>::from_sql(
            value,
        )?)),
        INT4 => DynamicValue::Int(i64::from(<i32 as FromSql<st::Integer, Pg>>::from_sql(
            value,
        )?)),
        INT8 => DynamicValue::Int(<i64 as FromSql<st::BigInt, Pg>>::from_sql(value)?),
        FLOAT4 => DynamicValue::Float(f64::from(<f32 as FromSql<st::Float, Pg>>::from_sql(value)?)),
        FLOAT8 => DynamicValue::Float(<f64 as FromSql<st::Double, Pg>>::from_sql(value)?),
        TEXT | VARCHAR | BPCHAR | NAME => {
            DynamicValue::Text(<String as FromSql<st::Text, Pg>>::from_sql(value)?)
        }
        BYTEA => DynamicValue::Bytes(<Vec<u8> as FromSql<st::Binary, Pg>>::from_sql(value)?),
        #[cfg(feature = "uuid")]
        UUID => DynamicValue::Uuid(<uuid::Uuid as FromSql<st::Uuid, Pg>>::from_sql(value)?),
        #[cfg(not(feature = "uuid"))]
        UUID => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "uuid",
                sql_type: "UUID",
            }
            .into())
        }
        #[cfg(feature = "numeric")]
        NUMERIC => DynamicValue::Numeric(
            <bigdecimal::BigDecimal as FromSql<st::Numeric, Pg>>::from_sql(value)?,
        ),
        #[cfg(not(feature = "numeric"))]
        NUMERIC => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "numeric",
                sql_type: "NUMERIC",
            }
            .into())
        }
        #[cfg(feature = "chrono")]
        TIMESTAMP => DynamicValue::Timestamp(<chrono::NaiveDateTime as FromSql<
            st::Timestamp,
            Pg,
        >>::from_sql(value)?),
        #[cfg(feature = "chrono")]
        TIMESTAMPTZ => DynamicValue::TimestampTz(<chrono::DateTime<chrono::Utc> as FromSql<
            st::Timestamptz,
            Pg,
        >>::from_sql(value)?),
        #[cfg(feature = "chrono")]
        DATE => DynamicValue::Date(<chrono::NaiveDate as FromSql<st::Date, Pg>>::from_sql(
            value,
        )?),
        #[cfg(feature = "chrono")]
        TIME => DynamicValue::Time(<chrono::NaiveTime as FromSql<st::Time, Pg>>::from_sql(
            value,
        )?),
        #[cfg(not(feature = "chrono"))]
        TIMESTAMP => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "TIMESTAMP",
            }
            .into())
        }
        #[cfg(not(feature = "chrono"))]
        TIMESTAMPTZ => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "TIMESTAMPTZ",
            }
            .into())
        }
        #[cfg(not(feature = "chrono"))]
        DATE => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "DATE",
            }
            .into())
        }
        #[cfg(not(feature = "chrono"))]
        TIME => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "TIME",
            }
            .into())
        }
        #[cfg(feature = "serde_json")]
        JSON => DynamicValue::Json(<serde_json::Value as FromSql<st::Json, Pg>>::from_sql(
            value,
        )?),
        #[cfg(feature = "serde_json")]
        JSONB => DynamicValue::Jsonb(<serde_json::Value as FromSql<st::Jsonb, Pg>>::from_sql(
            value,
        )?),
        #[cfg(not(feature = "serde_json"))]
        JSON => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "serde_json",
                sql_type: "JSON",
            }
            .into())
        }
        #[cfg(not(feature = "serde_json"))]
        JSONB => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "serde_json",
                sql_type: "JSONB",
            }
            .into())
        }
        other => {
            return Err(DynamicSchemaError::UnsupportedType {
                backend: "PostgreSQL",
                sql_type: format!("OID {other}"),
            }
            .into())
        }
    })
}

#[cfg(feature = "sqlite")]
impl FromSql<Any, diesel::sqlite::Sqlite> for DynamicValue {
    fn from_sql(value: diesel::sqlite::SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        sqlite_dynamic_value(value)
    }

    fn from_nullable_sql(
        value: Option<diesel::sqlite::SqliteValue<'_, '_, '_>>,
    ) -> deserialize::Result<Self> {
        match value {
            Some(value) => sqlite_dynamic_value(value),
            None => Ok(DynamicValue::Null),
        }
    }
}

#[cfg(feature = "sqlite")]
fn sqlite_dynamic_value(
    mut value: diesel::sqlite::SqliteValue<'_, '_, '_>,
) -> deserialize::Result<DynamicValue> {
    use diesel::sqlite::SqliteType;

    // SQLite reports storage classes, not declared types.
    // `SmallInt`, `Integer`, and `Float` are bind-only and matched for exhaustiveness.
    Ok(match value.value_type() {
        None => DynamicValue::Null,
        Some(SqliteType::SmallInt | SqliteType::Integer | SqliteType::Long) => {
            DynamicValue::Int(value.read_long())
        }
        Some(SqliteType::Float | SqliteType::Double) => DynamicValue::Float(value.read_double()),
        Some(SqliteType::Text) => DynamicValue::Text(value.read_text().to_owned()),
        Some(SqliteType::Binary) => DynamicValue::Bytes(value.read_blob().to_owned()),
    })
}

#[cfg(feature = "mysql")]
impl FromSql<Any, diesel::mysql::Mysql> for DynamicValue {
    fn from_sql(value: diesel::mysql::MysqlValue<'_>) -> deserialize::Result<Self> {
        mysql_like_dynamic_value::<diesel::mysql::Mysql>(value, "MySQL")
    }

    fn from_nullable_sql(
        value: Option<diesel::mysql::MysqlValue<'_>>,
    ) -> deserialize::Result<Self> {
        match value {
            Some(value) => mysql_like_dynamic_value::<diesel::mysql::Mysql>(value, "MySQL"),
            None => Ok(DynamicValue::Null),
        }
    }
}

#[cfg(feature = "mariadb")]
impl FromSql<Any, diesel::mariadb::Mariadb> for DynamicValue {
    fn from_sql(value: diesel::mariadb::MariadbValue<'_>) -> deserialize::Result<Self> {
        mysql_like_dynamic_value::<diesel::mariadb::Mariadb>(value, "MariaDB")
    }

    fn from_nullable_sql(
        value: Option<diesel::mariadb::MariadbValue<'_>>,
    ) -> deserialize::Result<Self> {
        match value {
            Some(value) => mysql_like_dynamic_value::<diesel::mariadb::Mariadb>(value, "MariaDB"),
            None => Ok(DynamicValue::Null),
        }
    }
}

#[cfg(any(feature = "mysql", feature = "mariadb"))]
fn mysql_like_dynamic_value<DB>(
    value: diesel::mysql_like::MysqlValue<'_>,
    backend: &'static str,
) -> deserialize::Result<DynamicValue>
where
    DB: diesel::mysql_like::MysqlLikeBackend,
{
    use diesel::mysql_like::MysqlType as T;
    use diesel::sql_types as st;

    let ty = value.value_type();
    Ok(match ty {
        T::Tiny => DynamicValue::Int(i64::from(<i8 as FromSql<st::TinyInt, DB>>::from_sql(
            value,
        )?)),
        T::Short => DynamicValue::Int(i64::from(<i16 as FromSql<st::SmallInt, DB>>::from_sql(
            value,
        )?)),
        T::Long => DynamicValue::Int(i64::from(<i32 as FromSql<st::Integer, DB>>::from_sql(
            value,
        )?)),
        T::LongLong => DynamicValue::Int(<i64 as FromSql<st::BigInt, DB>>::from_sql(value)?),
        T::UnsignedTiny => DynamicValue::UInt(u64::from(<u8 as FromSql<
            st::Unsigned<st::TinyInt>,
            DB,
        >>::from_sql(value)?)),
        T::UnsignedShort => DynamicValue::UInt(u64::from(<u16 as FromSql<
            st::Unsigned<st::SmallInt>,
            DB,
        >>::from_sql(value)?)),
        T::UnsignedLong => DynamicValue::UInt(u64::from(<u32 as FromSql<
            st::Unsigned<st::Integer>,
            DB,
        >>::from_sql(value)?)),
        T::UnsignedLongLong => DynamicValue::UInt(<u64 as FromSql<
            st::Unsigned<st::BigInt>,
            DB,
        >>::from_sql(value)?),
        T::Float => {
            DynamicValue::Float(f64::from(<f32 as FromSql<st::Float, DB>>::from_sql(value)?))
        }
        T::Double => DynamicValue::Float(<f64 as FromSql<st::Double, DB>>::from_sql(value)?),
        // MySQL reports JSON as `String`. MariaDB reports it as `Blob`.
        T::String | T::Enum | T::Set => {
            DynamicValue::Text(<String as FromSql<st::Text, DB>>::from_sql(value)?)
        }
        T::Blob => DynamicValue::Bytes(<Vec<u8> as FromSql<st::Binary, DB>>::from_sql(value)?),
        #[cfg(feature = "numeric")]
        T::Numeric => DynamicValue::Numeric(<bigdecimal::BigDecimal as FromSql<
            st::Numeric,
            DB,
        >>::from_sql(value)?),
        #[cfg(not(feature = "numeric"))]
        T::Numeric => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "numeric",
                sql_type: "NUMERIC",
            }
            .into())
        }
        #[cfg(feature = "chrono")]
        T::Date => DynamicValue::Date(<chrono::NaiveDate as FromSql<st::Date, DB>>::from_sql(
            value,
        )?),
        #[cfg(feature = "chrono")]
        T::DateTime => DynamicValue::Timestamp(<chrono::NaiveDateTime as FromSql<
            st::Datetime,
            DB,
        >>::from_sql(value)?),
        #[cfg(feature = "chrono")]
        T::Timestamp => DynamicValue::Timestamp(<chrono::NaiveDateTime as FromSql<
            st::Timestamp,
            DB,
        >>::from_sql(value)?),
        #[cfg(feature = "chrono")]
        T::Time => DynamicValue::Duration(mysql_time_to_duration(
            <diesel::mysql_like::data_types::MysqlTime as FromSql<st::Time, DB>>::from_sql(value)?,
        )?),
        #[cfg(not(feature = "chrono"))]
        T::Date => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "DATE",
            }
            .into())
        }
        #[cfg(not(feature = "chrono"))]
        T::DateTime => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "DATETIME",
            }
            .into())
        }
        #[cfg(not(feature = "chrono"))]
        T::Timestamp => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "TIMESTAMP",
            }
            .into())
        }
        #[cfg(not(feature = "chrono"))]
        T::Time => {
            return Err(DynamicSchemaError::FeatureDisabled {
                feature: "chrono",
                sql_type: "TIME",
            }
            .into())
        }
        // `MysqlType` is non-exhaustive, so unknown tags fail rather than guess.
        other => {
            return Err(DynamicSchemaError::UnsupportedType {
                backend,
                sql_type: format!("{other:?}"),
            }
            .into())
        }
    })
}

#[cfg(all(feature = "chrono", any(feature = "mysql", feature = "mariadb")))]
fn mysql_time_to_duration(
    time: diesel::mysql_like::data_types::MysqlTime,
) -> deserialize::Result<chrono::Duration> {
    use core::convert::TryFrom;

    let micros = i64::from(time.hour) * 3_600_000_000
        + i64::from(time.minute) * 60_000_000
        + i64::from(time.second) * 1_000_000
        + i64::try_from(time.second_part)?;
    let micros = if time.neg { -micros } else { micros };
    Ok(chrono::Duration::microseconds(micros))
}
