#![recursion_limit = "1024"]
// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![allow(
    clippy::needless_doctest_main,
    clippy::needless_pass_by_value,
    clippy::map_unwrap_or
)]
#![warn(
    clippy::wrong_pub_self_convention,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic, proc_macro_span))]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

mod diagnostic_shim;
mod field;
mod meta;
mod model;
mod resolved_at_shim;
mod util;

mod as_changeset;
mod as_expression;
mod associations;
mod diesel_numeric_ops;
mod from_sql_row;
mod identifiable;
mod insertable;
mod query_id;
mod queryable;
mod queryable_by_name;
mod selectable;
mod sql_function;
mod sql_type;
mod valid_grouping;

use diagnostic_shim::*;

/// Implements `AsChangeset`
///
/// To implement `AsChangeset` this derive needs to know the corresponding table
/// type. By default it uses the `snake_case` type name with an added `s` from
/// the current scope.
/// It is possible to change this default by using `#[table_name = "something"]`.
///
/// If a field name of your struct differs
/// from the name of the corresponding column, you can annotate the field with
/// `#[column_name = "some_column_name"]`.
///
/// By default, any `Option` fields on the struct are skipped if their value is
/// `None`. If you would like to assign `NULL` to the field instead, you can
/// annotate your struct with `#[changeset_options(treat_none_as_null =
/// "true")]`.
///
/// # Attributes
///
/// ## Optional container attributes
///
/// * `#[changeset_options(treat_none_as_null = "true")]`, specifies that
/// the derive should threat `None` values as `NULL`. By default
/// `Option::<T>::None` is just skipped. To insert a `NULL` using default
/// behavior use `Option::<Option<T>>::Some(None)`
/// * `#[table_name = "path::to::table"]`, specifies a path to the table for which the
/// current type is a changeset. The path is relative to the current module.
/// If this attribute is not used, the type name converted to
/// `snake_case` with an added `s` is used as table name.
///
/// ## Optional field attributes
///
/// * `#[column_name = "some_column_name"]`, overrides the column name
/// of the current field to `some_column_name`. By default the field
/// name is used as column name.
#[proc_macro_derive(
    AsChangeset,
    attributes(table_name, primary_key, column_name, changeset_options)
)]
pub fn derive_as_changeset(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, as_changeset::derive)
}

/// Implements all required variants of `AsExpression`
///
/// This derive will generate the following impls:
///
/// - `impl AsExpression<SqlType> for YourType`
/// - `impl AsExpression<Nullable<SqlType>> for YourType`
/// - `impl AsExpression<SqlType> for &'a YourType`
/// - `impl AsExpression<Nullable<SqlType>> for &'a YourType`
/// - `impl AsExpression<SqlType> for &'a &'b YourType`
/// - `impl AsExpression<Nullable<SqlType>> for &'a &'b YourType`
///
/// If your type is unsized,
/// you can specify this by adding the annotation `#[diesel(not_sized)]`
/// as attribute on the type. This will skip the impls for non-reference types.
///
/// # Attributes:
///
/// ## Required container attributes
///
/// * `#[sql_type = "SqlType"]`, to specify the sql type of the
///  generated implementations. If the attribute exists multiple times
///  impls for each sql type are generated.
///
/// ## Optional container attributes
///
/// * `#[diesel(not_sized)]`, to skip generating impls that require
///   that the type is `Sized`
#[proc_macro_derive(AsExpression, attributes(diesel, sql_type))]
pub fn derive_as_expression(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, as_expression::derive)
}

/// Implement required traits for the associations API
///
/// This derive implement support for diesels associations api. Check the
/// module level documentation of the `diesel::associations` module for details.
///
/// # Attributes
///
/// # Required container attributes
///
/// * `#[belongs_to(User)]`, to specify a child-to-parent relation ship
/// between the current type and the specified parent type (`User`).
/// If this attribute is given multiple times, multiple relation ships
/// are generated.
/// * `#[belongs_to(User, foreign_key = "mykey")]`, variant of the attribute
/// above. Allows to specify the name of the foreign key. If the foreign key
/// is not specified explicitly, the remote lower case type name with an
/// appended `_id` is used as foreign key name. (`user_id` in this example
/// case)
///
/// # Optional container attributes
///
/// * `#[table_name = "path::to::table"]` specifies a path to the table this
///    type belongs to. The path is relative to the current module.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name.
///
/// # Optional field attributes
///
/// * `#[column_name = "some_column_name"]`, overrides the column the current
/// field maps to to `some_column_name`. By default the field name is used
/// as column name. Only useful for the foreign key field.
///
#[proc_macro_derive(Associations, attributes(belongs_to, column_name, table_name))]
pub fn derive_associations(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, associations::derive)
}

/// Implement numeric operators for the current query node
#[proc_macro_derive(DieselNumericOps)]
pub fn derive_diesel_numeric_ops(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, diesel_numeric_ops::derive)
}

/// Implements `Queryable` for primitive types
///
/// This derive is mostly useful to implement support deserializing
/// into rust types not supported by diesel itself.
///
/// There are no options or special considerations needed for this derive.
#[proc_macro_derive(FromSqlRow, attributes(diesel))]
pub fn derive_from_sql_row(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, from_sql_row::derive)
}

/// Implements `Identifiable` for references of the current type
///
/// By default, the primary key field is assumed to be a single field called `id`.
/// If it's not, you can put `#[primary_key(your_id)]` on your struct.
/// If you have a composite primary key, the syntax is `#[primary_key(id1, id2)]`.
///
/// By default, `#[derive(Identifiable)]` will assume that your table is
/// in scope and its name is the plural form of your struct name.
/// Diesel uses very simple pluralization rules.
/// It only adds an `s` to the end, and converts `CamelCase` to `snake_case`.
/// If your table name does not follow this convention or is not in scope,
/// you can specify a path to the table with `#[table_name = "path::to::table"]`.
/// Our rules for inferring table names is considered public API.
/// It will never change without a major version bump.
///
/// # Attributes
///
/// ## Optional container attributes
///
/// * `#[table_name = "path::to::table"]` specifies a path to the table this
///    type belongs to. The path is relative to the current module.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name
/// * `#[primary_key(id1, id2)]` to specify the struct field that
///    that corresponds to the primary key. If not used, `id` will be
///    assumed as primary key field
#[proc_macro_derive(Identifiable, attributes(table_name, primary_key, column_name))]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, identifiable::derive)
}

/// Implements `Insertable`
///
/// To implement `Insertable` this derive needs to know the corresponding table
/// type. By default it uses the `snake_case` type name with an added `s`
/// from the current scope.
/// It is possible to change this default by using `#[table_name = "something"]`.
///
/// If a field name of your
/// struct differs from the name of the corresponding column,
/// you can annotate the field with `#[column_name = "some_column_name"]`.
///
/// Your struct can also contain fields which implement `Insertable`. This is
/// useful when you want to have one field map to more than one column (for
/// example, an enum that maps to a label and a value column). Add
/// `#[diesel(embed)]` to any such fields.
///
/// To provide custom serialization behavior for a field, you can use
/// `#[diesel(serialize_as = "SomeType")]`. If this attribute is present, Diesel
/// will call `.into` on the corresponding field and serialize the instance of `SomeType`,
/// rather than the actual field on your struct. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
/// Using `#[diesel(serialize_as)]` is **incompatible** with `#[diesel(embed)]`.
/// Normally, Diesel produces two implementations of the `Insertable` trait for your
/// struct using this derive: one for an owned version and one for a borrowed version.
/// Using `#[diesel(serialize_as)]` implies a conversion using `.into` which consumes the underlying value.
/// Hence, once you use `#[diesel(serialize_as)]`, Diesel can no longer insert borrowed
/// versions of your struct.
///
/// # Attributes
///
/// ## Optional container attributes
///
/// * `#[table_name = "path::to::table"]`, specifies a path to the table this type
/// is insertable into. The path is relative to the current module.
/// If this attribute is not used, the type name converted to
/// `snake_case` with an added `s` is used as table name
/// * `#[diesel(treat_none_as_default_value = "true/false")], specifies if `None` values
/// should be converted to `NULL` values on SQL side or treated as `DEFAULT_VALUE` primitive
/// *Note: This option may control if your query is stored in the
/// prepared statement cache or not*
///
/// ## Optional field attributes
///
/// * `#[column_name = "some_column_name"]`, overrides the column the current
/// field maps to `some_column_name`. By default the field name is used
/// as column name
/// * `#[diesel(embed)]`, specifies that the current field maps not only
/// to single database field, but is a struct that implements `Insertable`
/// * `#[diesel(serialize_as = "SomeType")]`, instead of serializing the actual
/// field type, Diesel will convert the field into `SomeType` using `.into` and
/// serialize that instead. By default this derive will serialize directly using
/// the actual field type.
///
/// # Examples
///
/// If we want to customize the serialization during insert, we can use `#[diesel(serialize_as)]`.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenv;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use diesel::{prelude::*, serialize::{ToSql, Output, self}, deserialize::{FromSqlRow}, expression::AsExpression, sql_types, backend::Backend};
/// # use schema::users;
/// # use std::io::Write;
/// #
/// #[derive(Debug, FromSqlRow, AsExpression)]
/// #[sql_type = "sql_types::Text"]
/// struct UppercaseString(pub String);
///
/// impl Into<UppercaseString> for String {
///     fn into(self) -> UppercaseString {
///         UppercaseString(self.to_uppercase())
///     }
/// }
///
/// impl<DB> ToSql<sql_types::Text, DB> for UppercaseString
///     where
///         DB: Backend,
///         String: ToSql<sql_types::Text, DB>,
/// {
///     fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
///         self.0.to_sql(out)
///     }
/// }
///
/// #[derive(Insertable, PartialEq, Debug)]
/// #[table_name = "users"]
/// struct InsertableUser {
///     id: i32,
///     #[diesel(serialize_as = "UppercaseString")]
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut connection_no_data();
/// #     connection.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name VARCHAR(255) NOT NULL)").unwrap();
/// let user = InsertableUser {
///     id: 1,
///     name: "thomas".to_string(),
/// };
///
/// diesel::insert_into(users)
///     .values(user)
///     .execute(connection)
///     .unwrap();
///
/// assert_eq!(
///     Ok("THOMAS".to_string()),
///     users.select(name).first(connection)
/// );
/// # Ok(())
/// # }
/// ```

#[proc_macro_derive(Insertable, attributes(table_name, column_name, diesel))]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, insertable::derive)
}

#[doc(hidden)]
#[proc_macro_derive(NonAggregate)]
pub fn derive_non_aggregate(input: TokenStream) -> TokenStream {
    eprintln!(
        "#[derive(NonAggregate)] is deprecated. Please use \
         `#[derive(ValidGrouping)]` instead.)"
    );
    expand_proc_macro(input, valid_grouping::derive)
}

/// Implements `QueryId`
///
/// For example, given this struct:
///
/// ```rust
/// # extern crate diesel;
/// #[derive(diesel::query_builder::QueryId)]
/// pub struct And<Left, Right> {
///     left: Left,
///     right: Right,
/// }
/// ```
///
/// the following implementation will be generated
///
/// ```rust
/// # extern crate diesel;
/// # struct And<Left, Right>(Left, Right);
/// # use diesel::query_builder::QueryId;
/// impl<Left, Right> QueryId for And<Left, Right>
/// where
///     Left: QueryId,
///     Right: QueryId,
/// {
///     type QueryId = And<Left::QueryId, Right::QueryId>;
///
///     const HAS_STATIC_QUERY_ID: bool = Left::HAS_STATIC_QUERY_ID && Right::HAS_STATIC_QUERY_ID;
/// }
/// ```
///
/// If the SQL generated by a struct is not uniquely identifiable by its type,
/// meaning that `HAS_STATIC_QUERY_ID` should always be false,
/// you should not derive this trait.
/// In that case you should implement it manually instead.
#[proc_macro_derive(QueryId)]
pub fn derive_query_id(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, query_id::derive)
}

/// Implements `Queryable` to load the result of statically typed queries
///
/// This trait can only be derived for structs, not enums.
///
/// **Note**: When this trait is derived, it will assume that __all fields on
/// your struct__ matches __all fields in the query__, including the order and
/// count. This means that field order is significant if you are using
/// `#[derive(Queryable)]`. __Field name has no effect__.
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = "SomeType")]`. If this attribute is present, Diesel
/// will deserialize the corresponding field into `SomeType`, rather than the
/// actual field type on your struct and then call
/// [`.try_into`](https://doc.rust-lang.org/stable/std/convert/trait.TryInto.html#tymethod.try_into)
/// to convert it to the actual field type. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
/// (Note: all types that have `Into<T>` automatically implement `TryInto<T>`,
/// for cases where your conversion is not faillible.)
///
/// # Attributes
///
///
/// ## Optional field attributes
///
/// * `#[diesel(deserialize_as = "Type")]`, instead of deserializing directly
///   into the field type, the implementation will deserialize into `Type`.
///   Then `Type` is converted via
///   [`.try_into`](https://doc.rust-lang.org/stable/std/convert/trait.TryInto.html#tymethod.try_into)
///   into the field type. By default this derive will deserialize directly into the field type
///
/// # Examples
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenv;
/// # include!("../../diesel/src/doctest_setup.rs");
/// #
/// #[derive(Queryable, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// let first_user = users.first(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// If we want to do additional work during deserialization, we can use
/// `deserialize_as` to use a different implementation.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenv;
/// # include!("../../diesel/src/doctest_setup.rs");
/// #
/// # use schema::users;
/// # use diesel::backend::{self, Backend};
/// # use diesel::deserialize::{self, Queryable, FromSql};
/// # use diesel::sql_types::Text;
/// #
/// struct LowercaseString(String);
///
/// impl Into<String> for LowercaseString {
///     fn into(self) -> String {
///         self.0
///     }
/// }
///
/// impl<DB> Queryable<Text, DB> for LowercaseString
/// where
///     DB: Backend,
///     String: FromSql<Text, DB>
/// {
///
///     type Row = String;
///
///     fn build(s: String) -> deserialize::Result<Self> {
///         Ok(LowercaseString(s.to_lowercase()))
///     }
/// }
///
/// #[derive(Queryable, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     #[diesel(deserialize_as = "LowercaseString")]
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// let first_user = users.first(connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// Alternatively, we can implement the trait for our struct manually.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenv;
/// # include!("../../diesel/src/doctest_setup.rs");
/// #
/// use schema::users;
/// use diesel::deserialize::{self, Queryable, FromSqlRow};
/// use diesel::row::Row;
///
/// # /*
/// type DB = diesel::sqlite::Sqlite;
/// # */
///
/// #[derive(PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// impl Queryable<users::SqlType, DB> for User
/// where
///    (i32, String): FromSqlRow<users::SqlType, DB>,
/// {
///     type Row = (i32, String);
///
///     fn build((id, name): Self::Row) -> deserialize::Result<Self> {
///         Ok(User { id, name: name.to_lowercase() })
///     }
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     use schema::users::dsl::*;
/// #     let connection = &mut establish_connection();
/// let first_user = users.first(connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
#[proc_macro_derive(Queryable, attributes(column_name, diesel))]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, queryable::derive)
}

/// Implements `QueryableByName` for untyped sql queries, such as that one generated
/// by `sql_query`
///
/// To derive this trait, Diesel needs to know the SQL type of each field. You
/// can do this by either annotating your struct with `#[table_name =
/// "some_table"]` (in which case the SQL type will be
/// `diesel::dsl::SqlTypeOf<table_name::column_name>`), or by annotating each
/// field with `#[sql_type = "SomeType"]`.
///
/// If the name of a field on your struct is different than the column in your
/// `table!` declaration, or if you are deriving this trait on a tuple struct,
/// you can annotate the field with `#[column_name = "some_column"]`. For tuple
/// structs, all fields must have this annotation.
///
/// If a field is another struct which implements `QueryableByName`,
/// instead of a column, you can annotate that struct with `#[diesel(embed)]`.
/// Then all fields contained by that inner struct are loaded into
/// the embedded struct.
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = "SomeType")]`. If this attribute is present, Diesel
/// will deserialize the corresponding field into `SomeType`, rather than the
/// actual field type on your struct and then call `.into` to convert it to the
/// actual field type. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
///
/// # Attributes
///
/// ## Type attributes
///
/// * `#[table_name = "path::to::table"]`, to specify that this type contains
///   columns for the specified table. The path is relative to the current module.
///   If no field attributes are specified the derive will use the sql type of
///   the corresponding column.
///
/// ## Field attributes
///
/// * `#[column_name = "some_column"]`, overrides the column name for
///    a given field. If not set, the name of the field is used as column
///    name. This attribute is required on tuple structs, if
///    `#[table_name = "some_table"]` is used, otherwise it's optional.
/// * `#[sql_type = "SomeType"]`, assumes `SomeType` as sql type of the
///    corresponding field. This attributes has precedence over all other
///    variants to specify the sql type.
/// * `#[diesel(deserialize_as = "Type")]`, instead of deserializing directly
///   into the field type, the implementation will deserialize into `Type`.
///   Then `Type` is converted via `.into()` into the field type. By default
///   this derive will deserialize directly into the field type
/// * `#[diesel(embed)]`, specifies that the current field maps not only
///   single database column, but is a type that implements
///   `QueryableByName` on it's own
///
/// # Examples
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenv;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use schema::users;
/// # use diesel::sql_query;
/// #
/// #[derive(QueryableByName, PartialEq, Debug)]
/// #[table_name = "users"]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// let first_user = sql_query("SELECT * FROM users ORDER BY id LIMIT 1")
///     .get_result(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// If we want to do additional work during deserialization, we can use
/// `deserialize_as` to use a different implementation.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenv;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use diesel::sql_query;
/// # use schema::users;
/// # use diesel::backend::{self, Backend};
/// # use diesel::deserialize::{self, FromSql};
/// #
/// struct LowercaseString(String);
///
/// impl Into<String> for LowercaseString {
///     fn into(self) -> String {
///         self.0
///     }
/// }
///
/// impl<DB, ST> FromSql<ST, DB> for LowercaseString
/// where
///     DB: Backend,
///     String: FromSql<ST, DB>,
/// {
///     fn from_sql(bytes: backend::RawValue<DB>) -> deserialize::Result<Self> {
///         String::from_sql(bytes)
///             .map(|s| LowercaseString(s.to_lowercase()))
///     }
/// }
///
/// #[derive(QueryableByName, PartialEq, Debug)]
/// #[table_name = "users"]
/// struct User {
///     id: i32,
///     #[diesel(deserialize_as = "LowercaseString")]
///     name: String,
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// let first_user = sql_query("SELECT * FROM users ORDER BY id LIMIT 1")
///     .get_result(connection)?;
/// let expected = User { id: 1, name: "sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
///
/// The custom derive generates impls similar to the follownig one
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenv;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use schema::users;
/// # use diesel::sql_query;
/// # use diesel::deserialize::{self, QueryableByName, FromSql};
/// # use diesel::row::NamedRow;
/// # use diesel::backend::Backend;
/// #
/// #[derive(PartialEq, Debug)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// impl<DB> QueryableByName<DB> for User
/// where
///     DB: Backend,
///     i32: FromSql<diesel::dsl::SqlTypeOf<users::id>, DB>,
///     String: FromSql<diesel::dsl::SqlTypeOf<users::name>, DB>,
/// {
///     fn build<'a>(row: &impl NamedRow<'a, DB>) -> deserialize::Result<Self> {
///         let id = NamedRow::get::<diesel::dsl::SqlTypeOf<users::id>, _>(row, "id")?;
///         let name = NamedRow::get::<diesel::dsl::SqlTypeOf<users::name>, _>(row, "name")?;
///
///         Ok(Self { id, name })
///     }
/// }
///
/// # fn main() {
/// #     run_test();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let connection = &mut establish_connection();
/// let first_user = sql_query("SELECT * FROM users ORDER BY id LIMIT 1")
///     .get_result(connection)?;
/// let expected = User { id: 1, name: "Sean".into() };
/// assert_eq!(expected, first_user);
/// #     Ok(())
/// # }
/// ```
#[proc_macro_derive(QueryableByName, attributes(table_name, column_name, sql_type, diesel))]
pub fn derive_queryable_by_name(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, queryable_by_name::derive)
}

/// Implements `Selectable`
///
/// To implement `Selectable` this derive needs to know the corresponding table
/// type. By default it uses the `snake_case` type name with an added `s`.
/// It is possible to change this default by using `#[table_name = "something"]`.
///
/// If the name of a field on your struct is different than the column in your
/// `table!` declaration, or if you are deriving this trait on a tuple struct,
/// you can annotate the field with `#[column_name = "some_column"]`. For tuple
/// structs, all fields must have this annotation.
///
/// If a field is another struct which implements `Selectable`,
/// instead of a column, you can annotate that struct with `#[diesel(embed)]`.
/// Then all fields contained by that inner struct are selected as separate tuple.
/// Fields from a inner struct can come from a different table, as long as the
/// select clause is valid in current query.
///
/// The derive enables using the `SelectableHelper::as_select` method to construct
/// select clauses, in order to use LoadDsl, you might also check the
/// `Queryable` trait and derive.
///
/// # Attributes
///
/// ## Type attributes
///
/// * `#[table_name = "path::to::table"]`, specifies a path to the table for which the
/// current type is selectable. The path is relative to the current module.
/// If this attribute is not used, the type name converted to
/// `snake_case` with an added `s` is used as table name.
///
/// ## Field attributes
/// * `#[column_name = "some_column"]`, overrides the column name for
///    a given field. If not set, the name of the field is used as column
///    name.
/// * `#[diesel(embed)]`, specifies that the current field maps not only
///    single database column, but is a type that implements
///    `Selectable` on it's own
#[proc_macro_derive(Selectable, attributes(table_name, column_name, sql_type, diesel))]
pub fn derive_selectable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, selectable::derive)
}

/// Implement necessary traits for adding a new sql type
///
/// This trait implements all necessary traits to define a
/// new sql type. This is useful for adding support for unsupported
/// or custom types on sql side. The sql type will be usable for
/// all backends you specified via the attributes listed below.
///
/// This derive will implement `NotNull`, `HasSqlType` and `SingleValue`.
/// When using this deriving,
/// you need to specify how the type is represented on various backends.
/// You don't need to specify every backend,
/// only the ones supported by your type.
///
/// For PostgreSQL, add  `#[postgres(type_name = "pg_type_name", type_schema = "pg_schema_name")]`
/// or `#[postgres(oid = "some_oid", array_oid = "some_oid")]` for
/// builtin types.
/// For MySQL, specify which variant of `MysqlType` should be used
/// by adding `#[mysql_type = "Variant"]`.
/// For SQLite, specify which variant of `SqliteType` should be used
/// by adding `#[sqlite_type = "Variant"]`.
///
/// # Attributes
///
/// ## Type attributes
///
/// * `#[postgres(type_name = "TypeName", type_schema = "public")]` specifies support for
/// a postgresql type with the name `TypeName` in the schema `public`. Prefer this variant
/// for types with no stable OID (== everything but the builtin types). It's possible to leaf
/// of the `type_schema` part. In that case diesel defaults to the default postgres search path.
/// * `#[postgres(oid = 42, array_oid = 142)]`, specifies support for a
/// postgresql type with the given `oid` and `array_oid`. This variant
/// should only be used with types that have a stable OID.
/// * `#[sqlite_type = "TypeName"]`, specifies support for a sqlite type
/// with the given name. `TypeName` needs to be one of the possible values
/// in `SqliteType`
/// * `#[mysql_type = "TypeName"]`, specifies support for a mysql type
/// with the given name. `TypeName` needs to be one of the possible values
/// in `MysqlType`
#[proc_macro_derive(SqlType, attributes(postgres, sqlite_type, mysql_type))]
pub fn derive_sql_type(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, sql_type::derive)
}

/// Implements `ValidGrouping`
///
/// This trait can be automatically derived for structs with no type parameters
/// which are never aggregate, as well as for structs which are `NonAggregate`
/// when all type parameters are `NonAggregate`. For example:
///
/// ```ignore
/// #[derive(ValidGrouping)]
/// struct LiteralOne;
///
/// #[derive(ValidGrouping)]
/// struct Plus<Lhs, Rhs>(Lhs, Rhs);
///
/// // The following impl will be generated:
///
/// impl<GroupByClause> ValidGrouping<GroupByClause> for LiteralOne {
///     type IsAggregate = is_aggregate::Never;
/// }
///
/// impl<Lhs, Rhs, GroupByClause> ValidGrouping<GroupByClause> for Plus<Lhs, Rhs>
/// where
///     Lhs: ValidGrouping<GroupByClause>,
///     Rhs: ValidGrouping<GroupByClause>,
///     Lhs::IsAggregate: MixedAggregates<Rhs::IsAggregate>,
/// {
///     type IsAggregate = <Lhs::IsAggregate as MixedAggregates<Rhs::IsAggregate>>::Output;
/// }
/// ```
///
/// For types which are always considered aggregate (such as an aggregate
/// function), annotate your struct with `#[diesel(aggregate)]` to set `IsAggregate`
/// explicitly to `is_aggregate::Yes`.
///
/// # Attributes
///
/// ## Optional container attributes
///
/// * `#[diesel(aggregate)]` for cases where the type represents an aggregating
///   SQL expression
#[proc_macro_derive(ValidGrouping, attributes(diesel))]
pub fn derive_valid_grouping(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, valid_grouping::derive)
}

/// Declare a sql function for use in your code.
///
/// Diesel only provides support for a very small number of SQL functions.
/// This macro enables you to add additional functions from the SQL standard,
/// as well as any custom functions your application might have.
///
/// The syntax for this macro is very similar to that of a normal Rust function,
/// except the argument and return types will be the SQL types being used.
/// Typically these types will come from [`diesel::sql_types`](../diesel/sql_types/index.html)
///
/// This macro will generate two items. A function with the name that you've
/// given, and a module with a helper type representing the return type of your
/// function. For example, this invocation:
///
/// ```ignore
/// sql_function!(fn lower(x: Text) -> Text);
/// ```
///
/// will generate this code:
///
/// ```ignore
/// pub fn lower<X>(x: X) -> lower::HelperType<X> {
///     ...
/// }
///
/// pub(crate) mod lower {
///     pub type HelperType<X> = ...;
/// }
/// ```
///
/// If you are using this macro for part of a library, where the function is
/// part of your public API, it is highly recommended that you re-export this
/// helper type with the same name as your function. This is the standard
/// structure:
///
/// ```ignore
/// pub mod functions {
///     use super::types::*;
///     use diesel::sql_types::*;
///
///     sql_function! {
///         /// Represents the Pg `LENGTH` function used with `tsvector`s.
///         fn length(x: TsVector) -> Integer;
///     }
/// }
///
/// pub mod helper_types {
///     /// The return type of `length(expr)`
///     pub type Length<Expr> = functions::length::HelperType<Expr>;
/// }
///
/// pub mod dsl {
///     pub use functions::*;
///     pub use helper_types::*;
/// }
/// ```
///
/// Most attributes given to this macro will be put on the generated function
/// (including doc comments).
///
/// # Adding Doc Comments
///
/// ```no_run
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// use diesel::sql_types::Text;
///
/// sql_function! {
///     /// Represents the `canon_crate_name` SQL function, created in
///     /// migration ....
///     fn canon_crate_name(a: Text) -> Text;
/// }
///
/// # fn main() {
/// # use self::crates::dsl::*;
/// let target_name = "diesel";
/// crates.filter(canon_crate_name(name).eq(canon_crate_name(target_name)));
/// // This will generate the following SQL
/// // SELECT * FROM crates WHERE canon_crate_name(crates.name) = canon_crate_name($1)
/// # }
/// ```
///
/// # Special Attributes
///
/// There are a handful of special attributes that Diesel will recognize. They
/// are:
///
/// - `#[aggregate]`
///   - Indicates that this is an aggregate function, and that `NonAggregate`
///     should not be implemented.
/// - `#[sql_name="name"]`
///   - The SQL to be generated is different than the Rust name of the function.
///     This can be used to represent functions which can take many argument
///     types, or to capitalize function names.
///
/// Functions can also be generic. Take the definition of `sum` for an example
/// of all of this:
///
/// ```no_run
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// use diesel::sql_types::Foldable;
///
/// sql_function! {
///     #[aggregate]
///     #[sql_name = "SUM"]
///     fn sum<ST: Foldable>(expr: ST) -> ST::Sum;
/// }
///
/// # fn main() {
/// # use self::crates::dsl::*;
/// crates.select(sum(id));
/// # }
/// ```
///
/// # SQL Functions without Arguments
///
/// A common example is ordering a query using the `RANDOM()` sql function,
/// which can be implemented using `sql_function!` like this:
///
/// ```rust
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// sql_function!(fn random() -> Text);
///
/// # fn main() {
/// # use self::crates::dsl::*;
/// crates.order(random());
/// # }
/// ```
///
/// # Use with SQLite
///
/// On most backends, the implementation of the function is defined in a
/// migration using `CREATE FUNCTION`. On SQLite, the function is implemented in
/// Rust instead. You must call `register_impl` or
/// `register_nondeterministic_impl` with every connection before you can use
/// the function.
///
/// These functions will only be generated if the `sqlite` feature is enabled,
/// and the function is not generic. Generic functions and variadic functions
/// are not supported on SQLite.
///
/// ```rust
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {
/// # }
/// #
/// use diesel::sql_types::{Integer, Double};
/// sql_function!(fn add_mul(x: Integer, y: Integer, z: Double) -> Double);
///
/// # #[cfg(feature = "sqlite")]
/// # fn run_test() -> Result<(), Box<::std::error::Error>> {
/// let connection = &mut SqliteConnection::establish(":memory:")?;
///
/// add_mul::register_impl(connection, |x: i32, y: i32, z: f64| {
///     (x + y) as f64 * z
/// })?;
///
/// let result = select(add_mul(1, 2, 1.5))
///     .get_result::<f64>(connection)?;
/// assert_eq!(4.5, result);
/// #     Ok(())
/// # }
/// ```
///
/// ## Panics
///
/// If an implementation of the custom function panics and unwinding is enabled, the panic is
/// caught and the function returns to libsqlite with an error. It cannot propagate the panics due
/// to the FFI bounary.
///
/// This is is the same for [custom aggregate functions](#custom-aggregate-functions).
///
/// ## Custom Aggregate Functions
///
/// Custom aggregate functions can be created in SQLite by adding an `#[aggregate]`
/// attribute inside of `sql_function`. `register_impl` needs to be called on
/// the generated function with a type implementing the
/// [SqliteAggregateFunction](../diesel/sqlite/trait.SqliteAggregateFunction.html)
/// trait as a type parameter as shown in the examples below.
///
/// ```rust
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// #   run().unwrap();
/// # }
/// #
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {
/// # }
/// use diesel::sql_types::Integer;
/// # #[cfg(feature = "sqlite")]
/// use diesel::sqlite::SqliteAggregateFunction;
///
/// sql_function! {
///     #[aggregate]
///     fn my_sum(x: Integer) -> Integer;
/// }
///
/// #[derive(Default)]
/// struct MySum { sum: i32 }
///
/// # #[cfg(feature = "sqlite")]
/// impl SqliteAggregateFunction<i32> for MySum {
///     type Output = i32;
///
///     fn step(&mut self, expr: i32) {
///         self.sum += expr;
///     }
///
///     fn finalize(aggregator: Option<Self>) -> Self::Output {
///         aggregator.map(|a| a.sum).unwrap_or_default()
///     }
/// }
/// # table! {
/// #     players {
/// #         id -> Integer,
/// #         score -> Integer,
/// #     }
/// # }
///
/// # #[cfg(feature = "sqlite")]
/// fn run() -> Result<(), Box<dyn (::std::error::Error)>> {
/// #    use self::players::dsl::*;
///     let connection = &mut SqliteConnection::establish(":memory:")?;
/// #    connection.execute("create table players (id integer primary key autoincrement, score integer)").unwrap();
/// #    connection.execute("insert into players (score) values (10), (20), (30)").unwrap();
///
///     my_sum::register_impl::<MySum, _>(connection)?;
///
///     let total_score = players.select(my_sum(score))
///         .get_result::<i32>(connection)?;
///
///     println!("The total score of all the players is: {}", total_score);
///
/// #    assert_eq!(60, total_score);
///     Ok(())
/// }
/// ```
///
/// With multiple function arguments the arguments are passed as a tuple to `SqliteAggregateFunction`
///
/// ```rust
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # #[cfg(feature = "sqlite")]
/// # fn main() {
/// #   run().unwrap();
/// # }
/// #
/// # #[cfg(not(feature = "sqlite"))]
/// # fn main() {
/// # }
/// use diesel::sql_types::{Float, Nullable};
/// # #[cfg(feature = "sqlite")]
/// use diesel::sqlite::SqliteAggregateFunction;
///
/// sql_function! {
///     #[aggregate]
///     fn range_max(x0: Float, x1: Float) -> Nullable<Float>;
/// }
///
/// #[derive(Default)]
/// struct RangeMax<T> { max_value: Option<T> }
///
/// # #[cfg(feature = "sqlite")]
/// impl<T: Default + PartialOrd + Copy + Clone> SqliteAggregateFunction<(T, T)> for RangeMax<T> {
///     type Output = Option<T>;
///
///     fn step(&mut self, (x0, x1): (T, T)) {
/// #        let max = if x0 >= x1 {
/// #            x0
/// #        } else {
/// #            x1
/// #        };
/// #
/// #        self.max_value = match self.max_value {
/// #            Some(current_max_value) if max > current_max_value => Some(max),
/// #            None => Some(max),
/// #            _ => self.max_value,
/// #        };
///         // Compare self.max_value to x0 and x1
///     }
///
///     fn finalize(aggregator: Option<Self>) -> Self::Output {
///         aggregator?.max_value
///     }
/// }
/// # table! {
/// #     student_avgs {
/// #         id -> Integer,
/// #         s1_avg -> Float,
/// #         s2_avg -> Float,
/// #     }
/// # }
///
/// # #[cfg(feature = "sqlite")]
/// fn run() -> Result<(), Box<dyn (::std::error::Error)>> {
/// #    use self::student_avgs::dsl::*;
///     let connection = &mut SqliteConnection::establish(":memory:")?;
/// #    connection.execute("create table student_avgs (id integer primary key autoincrement, s1_avg float, s2_avg float)").unwrap();
/// #    connection.execute("insert into student_avgs (s1_avg, s2_avg) values (85.5, 90), (79.8, 80.1)").unwrap();
///
///     range_max::register_impl::<RangeMax<f32>, _, _>(connection)?;
///
///     let result = student_avgs.select(range_max(s1_avg, s2_avg))
///         .get_result::<Option<f32>>(connection)?;
///
///     if let Some(max_semeseter_avg) = result {
///         println!("The largest semester average is: {}", max_semeseter_avg);
///     }
///
/// #    assert_eq!(Some(90f32), result);
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn sql_function_proc(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, sql_function::expand)
}

fn expand_proc_macro<T: syn::parse::Parse>(
    input: TokenStream,
    f: fn(T) -> Result<proc_macro2::TokenStream, Diagnostic>,
) -> TokenStream {
    let item = syn::parse(input).unwrap();
    match f(item) {
        Ok(x) => x.into(),
        Err(e) => {
            e.emit();
            "".parse().unwrap()
        }
    }
}
