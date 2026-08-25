//! Runtime row containers and dynamic value decoders.
//!
//! ```rust
//! # #[cfg(all(
//! #     feature = "sqlite",
//! #     feature = "chrono",
//! #     feature = "returning_clauses_for_sqlite_3_35"
//! # ))]
//! # fn main() -> diesel::QueryResult<()> {
//! use chrono::NaiveDate;
//! use diesel::deserialize::{self, FromSql};
//! use diesel::prelude::*;
//! use diesel::sql_types::{Date, Integer, Text};
//! use diesel_dynamic_schema::dynamic_value::{
//!     BackendDynamicValue, ChronoExtension, ChronoValue, DynamicDecodeContext, DynamicLoadDsl,
//!     DynamicValue, DynamicValueBackend, DynamicValueExtension,
//! };
//! use diesel_dynamic_schema::DynamicOutputClause;
//!
//! diesel::table! {
//!     doc_dynamic_users (id) {
//!         id -> Integer,
//!         name -> Text,
//!         born -> Date,
//!     }
//! }
//!
//! struct PrefixExtension;
//!
//! impl DynamicValueExtension<diesel::sqlite::Sqlite> for PrefixExtension {
//!     type Value = String;
//!
//!     fn claims(&self, context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>) -> bool {
//!         context
//!             .origin()
//!             .map(|origin| origin.column == "name")
//!             .unwrap_or(false)
//!     }
//!
//!     fn decode(
//!         &self,
//!         _context: &DynamicDecodeContext<'_, diesel::sqlite::Sqlite>,
//!         value: diesel::sqlite::SqliteValue<'_, '_, '_>,
//!     ) -> deserialize::Result<Self::Value> {
//!         <String as FromSql<Text, diesel::sqlite::Sqlite>>::from_sql(value)
//!             .map(|value| format!("user:{value}"))
//!     }
//! }
//!
//! fn is_common_text<DB, E>(value: &BackendDynamicValue<DB, E>) -> bool
//! where
//!     DB: DynamicValueBackend,
//! {
//!     matches!(value, DynamicValue::Text(_))
//! }
//!
//! let conn = &mut diesel::SqliteConnection::establish(":memory:").unwrap();
//! diesel::sql_query(
//!     "CREATE TABLE doc_dynamic_users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, born DATE NOT NULL)",
//! )
//! .execute(conn)?;
//! let born = NaiveDate::from_ymd_opt(2000, 1, 2).unwrap();
//! diesel::insert_into(doc_dynamic_users::table)
//!     .values((doc_dynamic_users::name.eq("Sean"), doc_dynamic_users::born.eq(born)))
//!     .execute(conn)?;
//!
//! let dynamic_users = diesel_dynamic_schema::table("doc_dynamic_users");
//! let dynamic_name = dynamic_users.column::<Text, _>("name");
//! let dynamic_born = dynamic_users.column::<Date, _>("born");
//!
//! let common_rows = dynamic_users
//!     .select(DynamicOutputClause::new().field(dynamic_name))
//!     .load_dynamic_values(conn)?;
//! assert!(is_common_text::<diesel::sqlite::Sqlite, core::convert::Infallible>(&common_rows[0][0]));
//!
//! let custom_rows = dynamic_users
//!     .select(DynamicOutputClause::new().field(dynamic_name))
//!     .load_dynamic_values_with(conn, &PrefixExtension)?;
//! assert_eq!(custom_rows[0][0], DynamicValue::Custom("user:Sean".into()));
//!
//! let date_rows = dynamic_users
//!     .select(DynamicOutputClause::new().field(dynamic_born))
//!     .load_dynamic_values_with(conn, &ChronoExtension)?;
//! assert_eq!(date_rows[0][0], DynamicValue::Custom(ChronoValue::Date(born)));
//!
//! let inserted = diesel::insert_into(doc_dynamic_users::table)
//!     .values((doc_dynamic_users::name.eq("Tess"), doc_dynamic_users::born.eq(born)))
//!     .returning(DynamicOutputClause::new().field(doc_dynamic_users::name))
//!     .get_dynamic_result(conn)?;
//! assert_eq!(inserted["name"], DynamicValue::Text("Tess".into()));
//! # Ok(())
//! # }
//! # #[cfg(not(all(
//! #     feature = "sqlite",
//! #     feature = "chrono",
//! #     feature = "returning_clauses_for_sqlite_3_35"
//! # )))]
//! # fn main() {}
//! # #[cfg(all(
//! #     feature = "sqlite",
//! #     feature = "chrono",
//! #     feature = "returning_clauses_for_sqlite_3_35"
//! # ))]
//! # main().unwrap();
//! # #[cfg(not(all(
//! #     feature = "sqlite",
//! #     feature = "chrono",
//! #     feature = "returning_clauses_for_sqlite_3_35"
//! # )))]
//! # main();
//! ```
//!
//! ```rust
//! # #[cfg(feature = "postgres")]
//! # fn main() -> diesel::QueryResult<()> {
//! use diesel::dsl::sql;
//! use diesel::prelude::*;
//! use diesel::sql_types::{Array, Integer};
//! use diesel_dynamic_schema::dynamic_value::{pg::PgBackendValue, DynamicLoadDsl, DynamicValue};
//! use diesel_dynamic_schema::DynamicOutputClause;
//!
//! # mod connection_setup {
//! #     include!("../../tests/connection_setup.rs");
//! # }
//! let conn = &mut connection_setup::establish_connection();
//! let output = DynamicOutputClause::new().field(sql::<Array<Integer>>("ARRAY[1,2]"));
//! let rows = diesel::select(output).load_dynamic_values(conn)?;
//!
//! match &rows[0][0] {
//!     DynamicValue::Backend(PgBackendValue::Array(array)) => {
//!         assert_eq!(array.dimensions()[0].length, 2);
//!         assert_eq!(array.values()[0], DynamicValue::Integer(1));
//!     }
//!     other => panic!("unexpected value: {:?}", other),
//! }
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "postgres"))]
//! # fn main() {}
//! # #[cfg(feature = "postgres")]
//! # main().unwrap();
//! # #[cfg(not(feature = "postgres"))]
//! # main();
//! ```

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::iter::FromIterator;
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

use diesel::backend::Backend;
use diesel::connection::{DefaultLoadingMode, LoadConnection};
use diesel::deserialize::{self, FromSql};
use diesel::expression::{QueryMetadata, TypedExpressionType};
use diesel::query_builder::{OutputFieldMetadata, Query, QueryFragment, QueryId};
use diesel::row::{Field, NamedRow, Row};
use diesel::{QueryResult, QueryableByName};

use crate::DynamicSchemaError;

/// A marker type for dynamic `FromSql` implementations.
pub struct Any;

pub(crate) mod private {
    pub trait DynamicValueBackendSeal {}

    #[cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes")]
    impl<DB> DynamicValueBackendSeal for DB where DB: diesel::backend::Backend {}
}

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

/// A dynamic value shared by all backends.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DynamicValue<B, E> {
    /// SQL `NULL`.
    Null,
    /// A boolean value.
    Bool(bool),
    /// A signed integer value.
    Integer(i64),
    /// An unsigned integer value.
    Unsigned(u64),
    /// A floating point value.
    Float(f64),
    /// A text value.
    Text(String),
    /// A byte value.
    Bytes(Vec<u8>),
    /// A backend value.
    Backend(B),
    /// A caller supplied value.
    Custom(E),
}

/// Backend support for dynamic values.
pub trait DynamicValueBackend: private::DynamicValueBackendSeal + Backend + Sized {
    /// Backend-only dynamic payload.
    type BackendValue<E>;
    /// Runtime type tag exposed to extensions.
    type TypeTag: Copy;

    /// Return the runtime tag for a value.
    fn dynamic_type_tag(value: Self::RawValue<'_>) -> Self::TypeTag;

    /// Decode a value with optional declared SQL type metadata.
    fn decode_dynamic_value<'a, F, Ext>(
        field: &F,
        context: &DynamicDecodeContext<'_, Self>,
        extension: &Ext,
    ) -> deserialize::Result<BackendDynamicValue<Self, Ext::Value>>
    where
        F: Field<'a, Self>,
        Ext: DynamicValueExtension<Self>;
}

/// The shared value type for a backend.
pub type BackendDynamicValue<DB, E> = DynamicValue<<DB as DynamicValueBackend>::BackendValue<E>, E>;

/// The shared value type with no caller extension.
pub type DefaultDynamicValue<DB> = BackendDynamicValue<DB, Infallible>;

/// A positional row loaded through `DynamicLoadDsl`.
pub type DynamicValueRow<DB, E> = DynamicRow<BackendDynamicValue<DB, E>>;

/// A named row loaded through `DynamicLoadDsl`.
pub type DynamicNamedRow<DB, E> = DynamicRow<NamedField<BackendDynamicValue<DB, E>>>;

/// Origin metadata for a returned column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicColumnOrigin<'a> {
    /// The schema name, if present.
    pub schema: Option<&'a str>,
    /// The table name.
    pub table: &'a str,
    /// The column name.
    pub column: &'a str,
}

/// Metadata passed to dynamic value extensions.
#[derive(Debug, Clone, Copy)]
pub struct DynamicDecodeContext<'a, DB>
where
    DB: DynamicValueBackend,
{
    field_name: Option<&'a str>,
    origin: Option<DynamicColumnOrigin<'a>>,
    declared_sql_type: Option<diesel::sql_types::SqlTypeDescriptor>,
    backend_tag: DB::TypeTag,
    pg_array_subscripts: Option<&'a [i32]>,
}

impl<'a, DB> DynamicDecodeContext<'a, DB>
where
    DB: DynamicValueBackend,
{
    fn new(
        field_name: Option<&'a str>,
        metadata: OutputFieldMetadata<'a>,
        backend_tag: DB::TypeTag,
    ) -> Self {
        Self {
            field_name,
            origin: metadata.origin.map(|origin| DynamicColumnOrigin {
                schema: origin.schema,
                table: origin.table,
                column: origin.column,
            }),
            declared_sql_type: metadata.declared_sql_type,
            backend_tag,
            pg_array_subscripts: None,
        }
    }

    #[cfg(feature = "postgres")]
    pub(crate) fn from_parts(
        field_name: Option<&'a str>,
        origin: Option<DynamicColumnOrigin<'a>>,
        declared_sql_type: Option<diesel::sql_types::SqlTypeDescriptor>,
        backend_tag: DB::TypeTag,
        pg_array_subscripts: Option<&'a [i32]>,
    ) -> Self {
        Self {
            field_name,
            origin,
            declared_sql_type,
            backend_tag,
            pg_array_subscripts,
        }
    }

    /// The returned field name.
    pub fn field_name(&self) -> Option<&'a str> {
        self.field_name
    }

    /// The direct column origin.
    pub fn origin(&self) -> Option<DynamicColumnOrigin<'a>> {
        self.origin
    }

    /// The declared SQL type.
    pub fn declared_sql_type(&self) -> Option<diesel::sql_types::SqlTypeDescriptor> {
        self.declared_sql_type
    }

    /// The backend runtime tag.
    pub fn backend_tag(&self) -> DB::TypeTag {
        self.backend_tag
    }

    /// PostgreSQL array subscripts for recursive array decoding.
    pub fn pg_array_subscripts(&self) -> Option<&'a [i32]> {
        self.pg_array_subscripts
    }
}

/// A borrowed extension for caller supplied dynamic values.
pub trait DynamicValueExtension<DB>
where
    DB: DynamicValueBackend,
{
    /// The value produced by this extension.
    type Value;

    /// Return whether this extension owns the current value.
    fn claims(&self, context: &DynamicDecodeContext<'_, DB>) -> bool;

    /// Decode a claimed value.
    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, DB>,
        value: DB::RawValue<'_>,
    ) -> deserialize::Result<Self::Value>;
}

impl<DB, Ext> DynamicValueExtension<DB> for &Ext
where
    DB: DynamicValueBackend,
    Ext: DynamicValueExtension<DB>,
{
    type Value = Ext::Value;

    fn claims(&self, context: &DynamicDecodeContext<'_, DB>) -> bool {
        (**self).claims(context)
    }

    fn decode(
        &self,
        context: &DynamicDecodeContext<'_, DB>,
        value: DB::RawValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        (**self).decode(context, value)
    }
}

/// Extension used by default dynamic loading.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoDynamicValueExtension;

impl<DB> DynamicValueExtension<DB> for NoDynamicValueExtension
where
    DB: DynamicValueBackend,
{
    type Value = Infallible;

    fn claims(&self, _context: &DynamicDecodeContext<'_, DB>) -> bool {
        false
    }

    fn decode(
        &self,
        _context: &DynamicDecodeContext<'_, DB>,
        _value: DB::RawValue<'_>,
    ) -> deserialize::Result<Self::Value> {
        Err("no dynamic extension claimed the value".into())
    }
}

/// A dynamically sized row.
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

/// A named dynamic field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamedField<I> {
    /// The field name.
    pub name: String,
    /// The field value.
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
    /// Return the field value at an index.
    pub fn get(&self, index: usize) -> Option<&I> {
        self.values.get(index)
    }

    /// Return the mutable field value at an index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut I> {
        self.values.get_mut(index)
    }

    /// Return the number of fields in this row.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return whether this row is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Return an iterator over row values.
    pub fn iter(&self) -> impl Iterator<Item = &I> {
        self.values.iter()
    }

    /// Return a mutable iterator over row values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut I> {
        self.values.iter_mut()
    }

    /// Create a dynamic row from a database row.
    pub fn from_row<'a, DB>(row: &impl Row<'a, DB>) -> deserialize::Result<Self>
    where
        DB: Backend,
        I: FromSql<Any, DB>,
    {
        let data = (0..row.field_count())
            .map(|i| {
                let field = Row::get(row, i).expect("checked field count");
                I::from_nullable_sql(field.value())
            })
            .collect::<deserialize::Result<_>>()?;

        Ok(Self { values: data })
    }
}

impl<I> DynamicRow<NamedField<I>> {
    /// Return the field value by name.
    pub fn get_by_name<S: AsRef<str>>(&self, name: S) -> Option<&I> {
        self.values
            .iter()
            .find(|f| f.name == name.as_ref())
            .map(|f| &f.value)
    }

    /// Return the mutable field value by name.
    pub fn get_mut_by_name<S: AsRef<str>>(&mut self, name: S) -> Option<&mut I> {
        self.values
            .iter_mut()
            .find(|f| f.name == name.as_ref())
            .map(|f| &mut f.value)
    }
}

impl<I> DynamicRow<NamedField<Option<I>>> {
    /// Create a nullable named dynamic row from a database row.
    pub fn from_nullable_row<'a, DB>(row: &impl Row<'a, DB>) -> deserialize::Result<Self>
    where
        DB: Backend,
        I: FromSql<Any, DB>,
    {
        let data = (0..row.field_count())
            .map(|i| {
                let field = Row::get(row, i).expect("checked field count");
                let value = match I::from_nullable_sql(field.value()) {
                    Ok(o) => Some(o),
                    Err(e) if e.is::<diesel::result::UnexpectedNullError>() => None,
                    Err(e) => return Err(e),
                };

                Ok(NamedField {
                    name: field
                        .field_name()
                        .ok_or(DynamicSchemaError::UnnamedField)?
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
                let field = Row::get(row, i).expect("checked field count");
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
                let field = Row::get(row, i).expect("checked field count");
                let value = I::from_nullable_sql(field.value())?;

                Ok(NamedField {
                    name: field
                        .field_name()
                        .ok_or(DynamicSchemaError::UnnamedField)?
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
            .expect("field not found")
    }
}

impl<'a, I> IndexMut<&'a str> for DynamicRow<NamedField<I>> {
    fn index_mut(&mut self, field_name: &'a str) -> &mut Self::Output {
        self.values
            .iter_mut()
            .find(|f| f.name == field_name)
            .map(|f| &mut f.value)
            .expect("field not found")
    }
}

impl<'a, I> Index<&'a String> for DynamicRow<NamedField<I>> {
    type Output = I;

    fn index(&self, field_name: &'a String) -> &Self::Output {
        self.index(field_name.as_str())
    }
}

impl<'a, I> IndexMut<&'a String> for DynamicRow<NamedField<I>> {
    fn index_mut(&mut self, field_name: &'a String) -> &mut Self::Output {
        self.index_mut(field_name.as_str())
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

/// Iterator returned by positional dynamic loading.
#[allow(missing_debug_implementations)]
pub struct DynamicValuesIter<'conn, 'query, 'ext, Conn, Ext, B = DefaultLoadingMode>
where
    Conn: LoadConnection<B> + 'conn,
    Conn::Backend: DynamicValueBackend,
    Ext: DynamicValueExtension<Conn::Backend>,
{
    cursor: Conn::Cursor<'conn, 'query>,
    metadata: Vec<OutputFieldMetadata<'query>>,
    extension: &'ext Ext,
    _marker: PhantomData<B>,
}

impl<'conn, 'query, 'ext, Conn, Ext, B> Iterator
    for DynamicValuesIter<'conn, 'query, 'ext, Conn, Ext, B>
where
    Conn: LoadConnection<B> + 'conn,
    Conn::Backend: DynamicValueBackend,
    Ext: DynamicValueExtension<Conn::Backend>,
{
    type Item = QueryResult<DynamicRow<BackendDynamicValue<Conn::Backend, Ext::Value>>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next().map(|row| {
            row.and_then(|row| {
                decode_dynamic_row(&row, &self.metadata, self.extension)
                    .map_err(diesel::result::Error::DeserializationError)
            })
        })
    }
}

/// Iterator returned by named dynamic loading.
#[allow(missing_debug_implementations)]
pub struct DynamicIter<'conn, 'query, 'ext, Conn, Ext, B = DefaultLoadingMode>
where
    Conn: LoadConnection<B> + 'conn,
    Conn::Backend: DynamicValueBackend,
    Ext: DynamicValueExtension<Conn::Backend>,
{
    inner: DynamicValuesIter<'conn, 'query, 'ext, Conn, Ext, B>,
}

impl<'conn, 'query, 'ext, Conn, Ext, B> Iterator for DynamicIter<'conn, 'query, 'ext, Conn, Ext, B>
where
    Conn: LoadConnection<B> + 'conn,
    Conn::Backend: DynamicValueBackend,
    Ext: DynamicValueExtension<Conn::Backend>,
{
    type Item = QueryResult<DynamicRow<NamedField<BackendDynamicValue<Conn::Backend, Ext::Value>>>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.cursor.next().map(|row| {
            row.and_then(|row| {
                decode_dynamic_named_row(&row, &self.inner.metadata, self.inner.extension)
                    .map_err(diesel::result::Error::DeserializationError)
            })
        })
    }
}

static NO_DYNAMIC_VALUE_EXTENSION: NoDynamicValueExtension = NoDynamicValueExtension;

/// Dynamic loading methods for metadata-aware values.
pub trait DynamicLoadDsl<DB>: Query + QueryFragment<DB> + QueryId + Sized
where
    DB: DynamicValueBackend,
{
    /// Load named dynamic rows as an iterator.
    fn load_dynamic_iter<'conn, 'query: 'conn, Conn>(
        &'query self,
        conn: &'conn mut Conn,
    ) -> QueryResult<DynamicIter<'conn, 'query, 'static, Conn, NoDynamicValueExtension>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
    {
        self.load_dynamic_iter_with(conn, &NO_DYNAMIC_VALUE_EXTENSION)
    }

    /// Load named dynamic rows as an iterator with an extension.
    fn load_dynamic_iter_with<'conn, 'query: 'conn, 'ext, Conn, Ext>(
        &'query self,
        conn: &'conn mut Conn,
        extension: &'ext Ext,
    ) -> QueryResult<DynamicIter<'conn, 'query, 'ext, Conn, Ext>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
        Ext: DynamicValueExtension<DB>,
    {
        Ok(DynamicIter {
            inner: self.load_dynamic_values_iter_with(conn, extension)?,
        })
    }

    /// Load named dynamic rows into a vector.
    fn load_dynamic<'conn, 'query: 'conn, Conn>(
        &'query self,
        conn: &'conn mut Conn,
    ) -> QueryResult<Vec<DynamicRow<NamedField<DefaultDynamicValue<DB>>>>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
    {
        self.load_dynamic_iter(conn)?.collect()
    }

    /// Load named dynamic rows into a vector with an extension.
    fn load_dynamic_with<'conn, 'query: 'conn, 'ext, Conn, Ext>(
        &'query self,
        conn: &'conn mut Conn,
        extension: &'ext Ext,
    ) -> QueryResult<Vec<DynamicNamedRow<DB, Ext::Value>>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
        Ext: DynamicValueExtension<DB>,
    {
        self.load_dynamic_iter_with(conn, extension)?.collect()
    }

    /// Load the first named dynamic row.
    fn get_dynamic_result<'conn, 'query: 'conn, Conn>(
        &'query self,
        conn: &'conn mut Conn,
    ) -> QueryResult<DynamicRow<NamedField<DefaultDynamicValue<DB>>>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
    {
        self.get_dynamic_result_with(conn, &NO_DYNAMIC_VALUE_EXTENSION)
    }

    /// Load the first named dynamic row with an extension.
    fn get_dynamic_result_with<'conn, 'query: 'conn, 'ext, Conn, Ext>(
        &'query self,
        conn: &'conn mut Conn,
        extension: &'ext Ext,
    ) -> QueryResult<DynamicNamedRow<DB, Ext::Value>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
        Ext: DynamicValueExtension<DB>,
    {
        match self.load_dynamic_iter_with(conn, extension)?.next() {
            Some(row) => row,
            None => Err(diesel::result::Error::NotFound),
        }
    }

    /// Load named dynamic rows for mutation-shaped call sites.
    fn get_dynamic_results<'conn, 'query: 'conn, Conn>(
        &'query self,
        conn: &'conn mut Conn,
    ) -> QueryResult<Vec<DynamicRow<NamedField<DefaultDynamicValue<DB>>>>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
    {
        self.load_dynamic(conn)
    }

    /// Load named dynamic rows for mutation-shaped call sites with an extension.
    fn get_dynamic_results_with<'conn, 'query: 'conn, 'ext, Conn, Ext>(
        &'query self,
        conn: &'conn mut Conn,
        extension: &'ext Ext,
    ) -> QueryResult<Vec<DynamicNamedRow<DB, Ext::Value>>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
        Ext: DynamicValueExtension<DB>,
    {
        self.load_dynamic_with(conn, extension)
    }

    /// Load positional dynamic rows as an iterator.
    fn load_dynamic_values_iter<'conn, 'query: 'conn, Conn>(
        &'query self,
        conn: &'conn mut Conn,
    ) -> QueryResult<DynamicValuesIter<'conn, 'query, 'static, Conn, NoDynamicValueExtension>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
    {
        self.load_dynamic_values_iter_with(conn, &NO_DYNAMIC_VALUE_EXTENSION)
    }

    /// Load positional dynamic rows as an iterator with an extension.
    fn load_dynamic_values_iter_with<'conn, 'query: 'conn, 'ext, Conn, Ext>(
        &'query self,
        conn: &'conn mut Conn,
        extension: &'ext Ext,
    ) -> QueryResult<DynamicValuesIter<'conn, 'query, 'ext, Conn, Ext>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
        Ext: DynamicValueExtension<DB>,
    {
        let mut metadata = Vec::new();
        self.collect_output_metadata(&mut metadata)?;
        let cursor = conn.load(self)?;
        Ok(DynamicValuesIter {
            cursor,
            metadata,
            extension,
            _marker: PhantomData,
        })
    }

    /// Load positional dynamic rows into a vector.
    fn load_dynamic_values<'conn, 'query: 'conn, Conn>(
        &'query self,
        conn: &'conn mut Conn,
    ) -> QueryResult<Vec<DynamicRow<DefaultDynamicValue<DB>>>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
    {
        self.load_dynamic_values_iter(conn)?.collect()
    }

    /// Load positional dynamic rows into a vector with an extension.
    fn load_dynamic_values_with<'conn, 'query: 'conn, 'ext, Conn, Ext>(
        &'query self,
        conn: &'conn mut Conn,
        extension: &'ext Ext,
    ) -> QueryResult<Vec<DynamicValueRow<DB, Ext::Value>>>
    where
        Self: 'query,
        Conn: LoadConnection<DefaultLoadingMode, Backend = DB>,
        DB: QueryMetadata<Self::SqlType>,
        Ext: DynamicValueExtension<DB>,
    {
        self.load_dynamic_values_iter_with(conn, extension)?
            .collect()
    }
}

impl<T, DB> DynamicLoadDsl<DB> for T
where
    DB: DynamicValueBackend,
    T: Query + QueryFragment<DB> + QueryId,
{
}

fn decode_dynamic_row<'a, DB, R, Ext>(
    row: &R,
    metadata: &[OutputFieldMetadata<'_>],
    extension: &Ext,
) -> deserialize::Result<DynamicRow<BackendDynamicValue<DB, Ext::Value>>>
where
    DB: DynamicValueBackend,
    R: Row<'a, DB>,
    Ext: DynamicValueExtension<DB>,
{
    if metadata.len() != row.field_count() {
        return Err(DynamicSchemaError::OutputFieldCountMismatch {
            metadata: metadata.len(),
            row: row.field_count(),
        }
        .into());
    }

    (0..row.field_count())
        .map(|idx| {
            let field = row.get(idx).expect("checked field count");
            decode_dynamic_field::<DB, _, _>(&field, metadata[idx], extension)
        })
        .collect::<deserialize::Result<Vec<_>>>()
        .map(DynamicRow::from)
}

fn decode_dynamic_named_row<'a, DB, R, Ext>(
    row: &R,
    metadata: &[OutputFieldMetadata<'_>],
    extension: &Ext,
) -> deserialize::Result<DynamicNamedRow<DB, Ext::Value>>
where
    DB: DynamicValueBackend,
    R: Row<'a, DB>,
    Ext: DynamicValueExtension<DB>,
{
    if metadata.len() != row.field_count() {
        return Err(DynamicSchemaError::OutputFieldCountMismatch {
            metadata: metadata.len(),
            row: row.field_count(),
        }
        .into());
    }

    (0..row.field_count())
        .map(|idx| {
            let field = row.get(idx).expect("checked field count");
            let name = field
                .field_name()
                .ok_or(DynamicSchemaError::UnnamedField)?
                .to_owned();
            decode_dynamic_field::<DB, _, _>(&field, metadata[idx], extension)
                .map(|value| NamedField { name, value })
        })
        .collect::<deserialize::Result<Vec<_>>>()
        .map(DynamicRow::from)
}

pub(crate) fn decode_dynamic_field<'a, DB, F, Ext>(
    field: &F,
    metadata: OutputFieldMetadata<'a>,
    extension: &Ext,
) -> deserialize::Result<BackendDynamicValue<DB, Ext::Value>>
where
    DB: DynamicValueBackend,
    F: Field<'a, DB>,
    Ext: DynamicValueExtension<DB>,
{
    let Some(value) = field.value() else {
        return Ok(DynamicValue::Null);
    };
    let backend_tag = DB::dynamic_type_tag(value);
    let context = DynamicDecodeContext::new(field.field_name(), metadata, backend_tag);

    if extension.claims(&context) {
        let value = field.value().ok_or(diesel::result::UnexpectedNullError)?;
        return extension.decode(&context, value).map(DynamicValue::Custom);
    }

    DB::decode_dynamic_value(field, &context, extension)
}

#[cfg(any(
    feature = "chrono",
    feature = "time",
    feature = "numeric",
    feature = "uuid",
    feature = "serde_json",
    feature = "network-address",
    feature = "ipnet-address"
))]
mod extensions;
#[cfg(any(
    feature = "chrono",
    feature = "time",
    feature = "numeric",
    feature = "uuid",
    feature = "serde_json",
    feature = "network-address",
    feature = "ipnet-address"
))]
pub use extensions::*;

#[cfg(any(feature = "mysql", feature = "mariadb"))]
pub mod mysql_like;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
pub mod sqlite;
