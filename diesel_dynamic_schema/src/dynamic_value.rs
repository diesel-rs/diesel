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
//! # #[cfg(feature = "sqlite")]
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
//! #
//! # fn result_main() -> QueryResult<()> {
//! #
//! # let conn = &mut connection_setup::establish_connection();
//! #
//! # // Create some example data by using typical SQL statements.
//! # connection_setup::create_user_table(conn);
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
//!    String(String),
//!    Integer(i32),
//! }
//!
//! # #[cfg(feature = "postgres")]
//! impl FromSql<Any, diesel::pg::Pg> for MyDynamicValue {
//!    fn from_sql(value: diesel::pg::PgValue) -> deserialize::Result<Self> {
//!        use diesel::pg::Pg;
//!        use std::num::NonZeroU32;
//!
//!        const VARCHAR_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1043) };
//!        const TEXT_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(25) };
//!        const INTEGER_OID: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(23) };
//!
//!        match value.get_oid() {
//!            VARCHAR_OID | TEXT_OID => {
//!                <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(value)
//!                    .map(MyDynamicValue::String)
//!            }
//!            INTEGER_OID => <i32 as FromSql<diesel::sql_types::Integer, Pg>>::from_sql(value)
//!                .map(MyDynamicValue::Integer),
//!            e => Err(format!("Unknown type: {}", e).into()),
//!        }
//!    }
//! }
//! ```

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::expression::TypedExpressionType;
use diesel::row::{Field, NamedRow, Row};
use diesel::QueryableByName;
use std::iter::FromIterator;
use std::ops::Index;

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

#[cfg(feature = "sqlite")]
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

/// A dynamically sized container that allows to receive
/// a not at compile time known number of columns from the database
#[derive(Debug)]
pub struct DynamicRow<I> {
    values: Vec<I>,
}

/// A helper struct used as field type in `DynamicRow`
/// to also return the name of the field along with the
/// value
#[derive(Debug, PartialEq, Eq)]
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

    /// Get the number of fields in the current row
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the current row is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
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

#[cfg(feature = "sqlite")]
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

#[cfg(feature = "sqlite")]
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

impl<'a, I> Index<&'a String> for DynamicRow<NamedField<I>> {
    type Output = I;

    fn index(&self, field_name: &'a String) -> &Self::Output {
        self.index(field_name as &str)
    }
}

impl<I> Index<String> for DynamicRow<NamedField<I>> {
    type Output = I;

    fn index(&self, field_name: String) -> &Self::Output {
        self.index(&field_name)
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
