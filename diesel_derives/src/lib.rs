// Clippy lints
#![allow(
    clippy::needless_doctest_main,
    clippy::needless_pass_by_value,
    clippy::map_unwrap_or
)]
#![warn(
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding,
    missing_copy_implementations
)]
#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic))]

extern crate diesel_table_macro_syntax;
extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::{parse_macro_input, parse_quote};

mod attrs;
mod deprecated;
mod field;
mod model;
mod parsers;
mod util;

mod as_changeset;
mod as_expression;
mod associations;
mod diesel_for_each_tuple;
mod diesel_numeric_ops;
mod diesel_public_if;
mod from_sql_row;
mod identifiable;
mod insertable;
mod multiconnection;
mod query_id;
mod queryable;
mod queryable_by_name;
mod selectable;
mod sql_function;
mod sql_type;
mod table;
mod valid_grouping;

/// Implements `AsChangeset`
///
/// To implement `AsChangeset` this derive needs to know the corresponding table
/// type. By default, it uses the `snake_case` type name with an added `s` from
/// the current scope.
/// It is possible to change this default by using `#[diesel(table_name = something)]`.
///
/// If a field name of your struct differs
/// from the name of the corresponding column, you can annotate the field with
/// `#[diesel(column_name = some_column_name)]`.
///
/// To provide custom serialization behavior for a field, you can use
/// `#[diesel(serialize_as = SomeType)]`. If this attribute is present, Diesel
/// will call `.into` on the corresponding field and serialize the instance of `SomeType`,
/// rather than the actual field on your struct. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
/// Normally, Diesel produces two implementations of the `AsChangeset` trait for your
/// struct using this derive: one for an owned version and one for a borrowed version.
/// Using `#[diesel(serialize_as)]` implies a conversion using `.into` which consumes the underlying value.
/// Hence, once you use `#[diesel(serialize_as)]`, Diesel can no longer insert borrowed
/// versions of your struct.
///
/// By default, any `Option` fields on the struct are skipped if their value is
/// `None`. If you would like to assign `NULL` to the field instead, you can
/// annotate your struct with `#[diesel(treat_none_as_null = true)]`.
///
/// # Attributes
///
/// ## Optional container attributes
///
/// * `#[diesel(treat_none_as_null = true)]`, specifies that
///    the derive should treat `None` values as `NULL`. By default
///    `Option::<T>::None` is just skipped. To insert a `NULL` using default
///    behavior use `Option::<Option<T>>::Some(None)`
/// * `#[diesel(table_name = path::to::table)]`, specifies a path to the table for which the
///    current type is a changeset. The path is relative to the current module.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name.
/// * `#[diesel(primary_key(id1, id2))]` to specify the struct field that
///    that corresponds to the primary key. If not used, `id` will be
///    assumed as primary key field
///
/// ## Optional field attributes
///
/// * `#[diesel(column_name = some_column_name)]`, overrides the column name
///    of the current field to `some_column_name`. By default, the field
///    name is used as column name.
/// * `#[diesel(serialize_as = SomeType)]`, instead of serializing the actual
///    field type, Diesel will convert the field into `SomeType` using `.into` and
///    serialize that instead. By default, this derive will serialize directly using
///    the actual field type.
/// * `#[diesel(treat_none_as_null = true/false)]`, overrides the container-level
///   `treat_none_as_null` attribute for the current field.
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(
        AsChangeset,
        attributes(diesel, table_name, column_name, primary_key, changeset_options)
    )
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(AsChangeset, attributes(diesel))
)]
pub fn derive_as_changeset(input: TokenStream) -> TokenStream {
    as_changeset::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
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
/// Using this derive requires implementing the `ToSql` trait for your type.
///
/// # Attributes:
///
/// ## Required container attributes
///
/// * `#[diesel(sql_type = SqlType)]`, to specify the sql type of the
///    generated implementations. If the attribute exists multiple times
///    impls for each sql type is generated.
///
/// ## Optional container attributes
///
/// * `#[diesel(not_sized)]`, to skip generating impls that require
///    that the type is `Sized`
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(AsExpression, attributes(diesel, sql_type))
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(AsExpression, attributes(diesel))
)]
pub fn derive_as_expression(input: TokenStream) -> TokenStream {
    as_expression::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implement required traits for the associations API
///
/// This derive implements support for Diesel's associations api. Check the
/// module level documentation of the `diesel::associations` module for details.
///
/// This derive generates the following impls:
/// * `impl BelongsTo<Parent> for YourType`
/// * `impl BelongsTo<&'a Parent> for YourType`
///
/// # Attributes
///
/// # Required container attributes
///
/// * `#[diesel(belongs_to(User))]`, to specify a child-to-parent relationship
///    between the current type and the specified parent type (`User`).
///    If this attribute is given multiple times, multiple relationships
///    are generated. `#[diesel(belongs_to(User, foreign_key = mykey))]` variant
///    allows us to specify the name of the foreign key. If the foreign key
///    is not specified explicitly, the remote lower case type name with
///    appended `_id` is used as a foreign key name. (`user_id` in this example
///    case)
///
/// # Optional container attributes
///
/// * `#[diesel(table_name = path::to::table)]` specifies a path to the table this
///    type belongs to. The path is relative to the current module.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name.
///
/// # Optional field attributes
///
/// * `#[diesel(column_name = some_column_name)]`, overrides the column the current
///    field maps to `some_column_name`. By default, the field name is used
///    as a column name.
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(Associations, attributes(diesel, belongs_to, column_name, table_name))
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(Associations, attributes(diesel, belongs_to, column_name, table_name))
)]
pub fn derive_associations(input: TokenStream) -> TokenStream {
    associations::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implement numeric operators for the current query node
#[proc_macro_derive(DieselNumericOps)]
pub fn derive_diesel_numeric_ops(input: TokenStream) -> TokenStream {
    diesel_numeric_ops::derive(parse_macro_input!(input)).into()
}

/// Implements `Queryable` for types that correspond to a single SQL type. The type must implement `FromSql`.
///
/// This derive is mostly useful to implement support deserializing
/// into rust types not supported by Diesel itself.
///
/// There are no options or special considerations needed for this derive.
#[proc_macro_derive(FromSqlRow, attributes(diesel))]
pub fn derive_from_sql_row(input: TokenStream) -> TokenStream {
    from_sql_row::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements `Identifiable` for references of the current type
///
/// By default, the primary key field is assumed to be a single field called `id`.
/// If it isn't, you can put `#[diesel(primary_key(your_id))]` on your struct.
/// If you have a composite primary key, the syntax is `#[diesel(primary_key(id1, id2))]`.
///
/// By default, `#[derive(Identifiable)]` will assume that your table is
/// in scope and its name is the plural form of your struct name.
/// Diesel uses basic pluralization rules.
/// It only adds an `s` to the end, and converts `CamelCase` to `snake_case`.
/// If your table name doesn't follow this convention or is not in scope,
/// you can specify a path to the table with `#[diesel(table_name = path::to::table)]`.
/// Our rules for inferring table names are considered public API.
/// It will never change without a major version bump.
///
/// This derive generates the following impls:
/// * `impl Identifiable for &'a YourType`
/// * `impl Identifiable for &'_ &'a YourType`
///
/// # Attributes
///
/// ## Optional container attributes
///
/// * `#[diesel(table_name = path::to::table)]` specifies a path to the table this
///    type belongs to. The path is relative to the current module.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name
/// * `#[diesel(primary_key(id1, id2))]` to specify the struct field that
///    that corresponds to the primary key. If not used, `id` will be
///    assumed as primary key field
///
/// # Optional field attributes
///
/// * `#[diesel(column_name = some_column_name)]`, overrides the column the current
///    field maps to `some_column_name`. By default, the field name is used
///    as a column name.
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(Identifiable, attributes(diesel, table_name, column_name, primary_key))
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(Identifiable, attributes(diesel))
)]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    identifiable::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements `Insertable`
///
/// To implement `Insertable` this derive needs to know the corresponding table
/// type. By default, it uses the `snake_case` type name with an added `s`
/// from the current scope.
/// It is possible to change this default by using `#[diesel(table_name = something)]`.
/// If `table_name` attribute is given multiple times, impls for each table are generated.
///
/// If a field name of your
/// struct differs from the name of the corresponding column,
/// you can annotate the field with `#[diesel(column_name = some_column_name)]`.
///
/// Your struct can also contain fields which implement `Insertable`. This is
/// useful when you want to have one field map to more than one column (for
/// example, an enum that maps to a label and a value column). Add
/// `#[diesel(embed)]` to any such fields.
///
/// To provide custom serialization behavior for a field, you can use
/// `#[diesel(serialize_as = SomeType)]`. If this attribute is present, Diesel
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
/// * `#[diesel(table_name = path::to::table)]`, specifies a path to the table this type
///    is insertable into. The path is relative to the current module.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name
/// * `#[diesel(treat_none_as_default_value = false)]`, specifies that `None` values
///    should be converted to `NULL` values on the SQL side instead of being treated as `DEFAULT`
///    value primitive. *Note*: This option may control if your query is stored in the
///    prepared statement cache or not*
///
/// ## Optional field attributes
///
/// * `#[diesel(column_name = some_column_name)]`, overrides the column the current
///    field maps to `some_column_name`. By default, the field name is used
///    as column name
/// * `#[diesel(embed)]`, specifies that the current field maps not only
///    to a single database field, but is a struct that implements `Insertable`
/// * `#[diesel(serialize_as = SomeType)]`, instead of serializing the actual
///    field type, Diesel will convert the field into `SomeType` using `.into` and
///    serialize that instead. By default, this derive will serialize directly using
///    the actual field type.
/// * `#[diesel(treat_none_as_default_value = true/false)]`, overrides the container-level
///   `treat_none_as_default_value` attribute for the current field.
/// * `#[diesel(skip_insertion)]`, skips insertion of this field. Useful for working with
///    generated columns.
///
/// # Examples
///
/// If we want to customize the serialization during insert, we can use `#[diesel(serialize_as)]`.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenvy;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use diesel::{prelude::*, serialize::{ToSql, Output, self}, deserialize::{FromSqlRow}, expression::AsExpression, sql_types, backend::Backend};
/// # use schema::users;
/// # use std::io::Write;
/// #
/// #[derive(Debug, FromSqlRow, AsExpression)]
/// #[diesel(sql_type = sql_types::Text)]
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
///     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
///         self.0.to_sql(out)
///     }
/// }
///
/// #[derive(Insertable, PartialEq, Debug)]
/// #[diesel(table_name = users)]
/// struct InsertableUser {
///     id: i32,
///     #[diesel(serialize_as = UppercaseString)]
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
/// #     diesel::sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY, name VARCHAR(255) NOT NULL)")
/// #         .execute(connection)
/// #         .unwrap();
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
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(Insertable, attributes(diesel, table_name, column_name))
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(Insertable, attributes(diesel))
)]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    insertable::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
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
/// you shouldn't derive this trait.
/// In that case, you should implement it manually instead.
#[proc_macro_derive(QueryId)]
pub fn derive_query_id(input: TokenStream) -> TokenStream {
    query_id::derive(parse_macro_input!(input)).into()
}

/// Implements `Queryable` to load the result of statically typed queries
///
/// This trait can only be derived for structs, not enums.
///
/// **Note**: When this trait is derived, it will assume that __all fields on
/// your struct__ matches __all fields in the query__, including the order and
/// count. This means that field order is significant if you're using
/// `#[derive(Queryable)]`. __Field name has no effect__. If you see errors while
/// loading data into a struct that derives `Queryable`: Consider using [`#[derive(Selectable)]`]
/// + `#[diesel(check_for_backend(YourBackendType))]` to check for mismatching fields at
/// compile-time.
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = SomeType)]`. If this attribute is present, Diesel
/// will deserialize the corresponding field into `SomeType`, rather than the
/// actual field type on your struct and then call
/// [`.try_into`](https://doc.rust-lang.org/stable/std/convert/trait.TryInto.html#tymethod.try_into)
/// to convert it to the actual field type. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
/// (Note: all types that have `Into<T>` automatically implement `TryInto<T>`,
/// for cases where your conversion is not fallible.)
///
/// # Attributes
///
/// ## Optional field attributes
///
/// * `#[diesel(deserialize_as = Type)]`, instead of deserializing directly
///    into the field type, the implementation will deserialize into `Type`.
///    Then `Type` is converted via
///    [`.try_into`](https://doc.rust-lang.org/stable/std/convert/trait.TryInto.html#tymethod.try_into)
///    into the field type. By default, this derive will deserialize directly into the field type
///
/// # Examples
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenvy;
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
/// # extern crate dotenvy;
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
///     #[diesel(deserialize_as = LowercaseString)]
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
/// # extern crate dotenvy;
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
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(Queryable, attributes(diesel, column_name))
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(Queryable, attributes(diesel))
)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    queryable::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements `QueryableByName` for untyped sql queries, such as that one generated
/// by `sql_query`
///
/// To derive this trait, Diesel needs to know the SQL type of each field.
/// It can get the data from the corresponding table type.
/// It uses the `snake_case` type name with an added `s`.
/// It is possible to change this default by using `#[diesel(table_name = something)]`.
/// If you define use the table type, the SQL type will be
/// `diesel::dsl::SqlTypeOf<table_name::column_name>`. In cases which there are no table type,
/// you can do the same by annotating each field with `#[diesel(sql_type = SomeType)]`.
///
/// If the name of a field on your struct is different from the column in your
/// `table!` declaration, or if you're deriving this trait on a tuple struct,
/// you can annotate the field with `#[diesel(column_name = some_column)]`. For tuple
/// structs, all fields must have this annotation.
///
/// If a field is another struct which implements `QueryableByName`,
/// instead of a column, you can annotate that with `#[diesel(embed)]`.
/// Then all fields contained by that inner struct are loaded into the embedded struct.
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = SomeType)]`. If this attribute is present, Diesel
/// will deserialize the corresponding field into `SomeType`, rather than the
/// actual field type on your struct and then call `.into` to convert it to the
/// actual field type. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
///
/// # Attributes
///
/// ## Optional container attributes
///
/// * `#[diesel(table_name = path::to::table)]`, to specify that this type contains
///    columns for the specified table. The path is relative to the current module.
///    If no field attributes are specified the derive will use the sql type of
///    the corresponding column.
/// * `#[diesel(check_for_backend(diesel::pg::Pg, diesel::mysql::Mysql))]`, instructs
///    the derive to generate additional code to identify potential type mismatches.
///    It accepts a list of backend types to check the types against. Using this option
///    will result in much better error messages in cases where some types in your `QueryableByName`
///    struct don't match. You need to specify the concrete database backend
///    this specific struct is indented to be used with, as otherwise rustc can't correctly
///    identify the required deserialization implementation.
///
/// ## Optional field attributes
///
/// * `#[diesel(column_name = some_column)]`, overrides the column name for
///    a given field. If not set, the name of the field is used as a column
///    name. This attribute is required on tuple structs, if
///    `#[diesel(table_name = some_table)]` is used, otherwise it's optional.
/// * `#[diesel(sql_type = SomeType)]`, assumes `SomeType` as sql type of the
///    corresponding field. These attributes have precedence over all other
///    variants to specify the sql type.
/// * `#[diesel(deserialize_as = Type)]`, instead of deserializing directly
///    into the field type, the implementation will deserialize into `Type`.
///    Then `Type` is converted via `.into()` into the field type. By default,
///    this derive will deserialize directly into the field type
/// * `#[diesel(embed)]`, specifies that the current field maps not only
///    a single database column, but it is a type that implements
///    `QueryableByName` on its own
///
/// # Examples
///
/// If we just want to map a query to our struct, we can use `derive`.
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenvy;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use schema::users;
/// # use diesel::sql_query;
/// #
/// #[derive(QueryableByName, PartialEq, Debug)]
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
/// # extern crate dotenvy;
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
///     fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
///         String::from_sql(bytes)
///             .map(|s| LowercaseString(s.to_lowercase()))
///     }
/// }
///
/// #[derive(QueryableByName, PartialEq, Debug)]
/// struct User {
///     id: i32,
///     #[diesel(deserialize_as = LowercaseString)]
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
/// The custom derive generates impls similar to the following one
///
/// ```rust
/// # extern crate diesel;
/// # extern crate dotenvy;
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
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(QueryableByName, attributes(diesel, table_name, column_name, sql_type))
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(QueryableByName, attributes(diesel))
)]
pub fn derive_queryable_by_name(input: TokenStream) -> TokenStream {
    queryable_by_name::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implements `Selectable`
///
/// To implement `Selectable` this derive needs to know the corresponding table
/// type. By default, it uses the `snake_case` type name with an added `s`.
/// It is possible to change this default by using `#[diesel(table_name = something)]`.
///
/// If the name of a field on your struct is different from the column in your
/// `table!` declaration, or if you're deriving this trait on a tuple struct,
/// you can annotate the field with `#[diesel(column_name = some_column)]`. For tuple
/// structs, all fields must have this annotation.
///
/// If a field is another struct which implements `Selectable`,
/// instead of a column, you can annotate that with `#[diesel(embed)]`.
/// Then all fields contained by that inner struct are selected as separate tuple.
/// Fields from an inner struct can come from a different table, as long as the
/// select clause is valid in the current query.
///
/// The derive enables using the `SelectableHelper::as_select` method to construct
/// select clauses, in order to use LoadDsl, you might also check the
/// `Queryable` trait and derive.
///
/// # Attributes
///
/// ## Type attributes
///
/// * `#[diesel(table_name = path::to::table)]`, specifies a path to the table for which the
///    current type is selectable. The path is relative to the current module.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name.
///
/// ## Optional Type attributes
///
/// * `#[diesel(check_for_backend(diesel::pg::Pg, diesel::mysql::Mysql))]`, instructs
///    the derive to generate additional code to identify potential type mismatches.
///    It accepts a list of backend types to check the types against. Using this option
///    will result in much better error messages in cases where some types in your `Queryable`
///    struct don't match. You need to specify the concrete database backend
///    this specific struct is indented to be used with, as otherwise rustc can't correctly
///    identify the required deserialization implementation.
///
/// ## Field attributes
///
/// * `#[diesel(column_name = some_column)]`, overrides the column name for
///    a given field. If not set, the name of the field is used as column
///    name.
/// * `#[diesel(embed)]`, specifies that the current field maps not only
///    a single database column, but is a type that implements
///    `Selectable` on its own
/// * `#[diesel(select_expression = some_custom_select_expression)]`, overrides
///   the entire select expression for the given field. It may be used to select with
///   custom tuples, or specify `select_expression = my_table::some_field.is_not_null()`,
///   or separate tables...
///   It may be used in conjunction with `select_expression_type` (described below)
/// * `#[diesel(select_expression_type = the_custom_select_expression_type]`, should be used
///   in conjunction with `select_expression` (described above) if the type is too complex
///   for diesel to infer it automatically. This will be required if select_expression is a custom
///   function call that doesn't have the corresponding associated type defined at the same path.
///   Example use (this would actually be inferred):
///   `#[diesel(select_expression_type = dsl::IsNotNull<my_table::some_field>)]`
#[proc_macro_derive(Selectable, attributes(diesel))]
pub fn derive_selectable(input: TokenStream) -> TokenStream {
    selectable::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Implement necessary traits for adding a new sql type
///
/// This trait implements all necessary traits to define a
/// new sql type. This is useful for adding support for unsupported
/// or custom types on the sql side. The sql type will be usable for
/// all backends you specified via the attributes listed below.
///
/// This derive will implement `NotNull`, `HasSqlType` and `SingleValue`.
/// When using this derive macro,
/// you need to specify how the type is represented on various backends.
/// You don't need to specify every backend,
/// only the ones supported by your type.
///
/// For PostgreSQL, add `#[diesel(postgres_type(name = "pg_type_name", schema = "pg_schema_name"))]`
/// or `#[diesel(postgres_type(oid = "some_oid", array_oid = "some_oid"))]` for
/// builtin types.
/// For MySQL, specify which variant of `MysqlType` should be used
/// by adding `#[diesel(mysql_type(name = "Variant"))]`.
/// For SQLite, specify which variant of `SqliteType` should be used
/// by adding `#[diesel(sqlite_type(name = "Variant"))]`.
///
/// # Attributes
///
/// ## Type attributes
///
/// * `#[diesel(postgres_type(name = "TypeName", schema = "public"))]` specifies support for
///    a postgresql type with the name `TypeName` in the schema `public`. Prefer this variant
///    for types with no stable OID (== everything but the builtin types). It is possible to leaf
///    of the `schema` part. In that case, Diesel defaults to the default postgres search path.
/// * `#[diesel(postgres_type(oid = 42, array_oid = 142))]`, specifies support for a
///    postgresql type with the given `oid` and `array_oid`. This variant
///    should only be used with types that have a stable OID.
/// * `#[diesel(sqlite_type(name = "TypeName"))]`, specifies support for a sqlite type
///    with the given name. `TypeName` needs to be one of the possible values
///    in `SqliteType`
/// * `#[diesel(mysql_type(name = "TypeName"))]`, specifies support for a mysql type
///    with the given name. `TypeName` needs to be one of the possible values
///    in `MysqlType`
#[cfg_attr(
    all(not(feature = "without-deprecated"), feature = "with-deprecated"),
    proc_macro_derive(SqlType, attributes(diesel, postgres, sqlite_type, mysql_type))
)]
#[cfg_attr(
    any(feature = "without-deprecated", not(feature = "with-deprecated")),
    proc_macro_derive(SqlType, attributes(diesel))
)]
pub fn derive_sql_type(input: TokenStream) -> TokenStream {
    sql_type::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
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
///    SQL expression
#[proc_macro_derive(ValidGrouping, attributes(diesel))]
pub fn derive_valid_grouping(input: TokenStream) -> TokenStream {
    valid_grouping::derive(parse_macro_input!(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declare a sql function for use in your code.
///
/// Diesel only provides support for a very small number of SQL functions.
/// This macro enables you to add additional functions from the SQL standard,
/// as well as any custom functions your application might have.
///
/// The syntax for this macro is very similar to that of a normal Rust function,
/// except the argument and return types will be the SQL types being used.
/// Typically, these types will come from [`diesel::sql_types`](../diesel/sql_types/index.html)
///
/// This macro will generate two items. A function with the name that you've
/// given, and a module with a helper type representing the return type of your
/// function. For example, this invocation:
///
/// ```ignore
/// define_sql_function!(fn lower(x: Text) -> Text);
/// ```
///
/// will generate this code:
///
/// ```ignore
/// pub fn lower<X>(x: X) -> lower<X> {
///     ...
/// }
///
/// pub type lower<X> = ...;
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
/// define_sql_function! {
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
///     shouldn't be implemented.
/// - `#[sql_name = "name"]`
///   - The SQL to be generated is different from the Rust name of the function.
///     This can be used to represent functions which can take many argument
///     types, or to capitalize function names.
///
/// Functions can also be generic. Take the definition of `sum`, for example:
///
/// ```no_run
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// use diesel::sql_types::Foldable;
///
/// define_sql_function! {
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
/// which can be implemented using `define_sql_function!` like this:
///
/// ```rust
/// # extern crate diesel;
/// # use diesel::*;
/// #
/// # table! { crates { id -> Integer, name -> VarChar, } }
/// #
/// define_sql_function!(fn random() -> Text);
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
/// `register_nondeterministic_impl` (in the generated function's `_internals`
/// module) with every connection before you can use the function.
///
/// These functions will only be generated if the `sqlite` feature is enabled,
/// and the function is not generic.
/// SQLite doesn't support generic functions and variadic functions.
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
/// define_sql_function!(fn add_mul(x: Integer, y: Integer, z: Double) -> Double);
///
/// # #[cfg(feature = "sqlite")]
/// # fn run_test() -> Result<(), Box<dyn std::error::Error>> {
/// let connection = &mut SqliteConnection::establish(":memory:")?;
///
/// add_mul_utils::register_impl(connection, |x: i32, y: i32, z: f64| {
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
/// caught and the function returns to libsqlite with an error. It can't propagate the panics due
/// to the FFI boundary.
///
/// This is the same for [custom aggregate functions](#custom-aggregate-functions).
///
/// ## Custom Aggregate Functions
///
/// Custom aggregate functions can be created in SQLite by adding an `#[aggregate]`
/// attribute inside `define_sql_function`. `register_impl` (in the generated function's `_utils`
/// module) needs to be called with a type implementing the
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
/// define_sql_function! {
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
/// #    diesel::sql_query("create table players (id integer primary key autoincrement, score integer)")
/// #        .execute(connection)
/// #        .unwrap();
/// #    diesel::sql_query("insert into players (score) values (10), (20), (30)")
/// #        .execute(connection)
/// #        .unwrap();
///
///     my_sum_utils::register_impl::<MySum, _>(connection)?;
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
/// With multiple function arguments, the arguments are passed as a tuple to `SqliteAggregateFunction`
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
/// define_sql_function! {
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
/// #    diesel::sql_query("create table student_avgs (id integer primary key autoincrement, s1_avg float, s2_avg float)")
/// #       .execute(connection)
/// #       .unwrap();
/// #    diesel::sql_query("insert into student_avgs (s1_avg, s2_avg) values (85.5, 90), (79.8, 80.1)")
/// #        .execute(connection)
/// #        .unwrap();
///
///     range_max_utils::register_impl::<RangeMax<f32>, _, _>(connection)?;
///
///     let result = student_avgs.select(range_max(s1_avg, s2_avg))
///         .get_result::<Option<f32>>(connection)?;
///
///     if let Some(max_semester_avg) = result {
///         println!("The largest semester average is: {}", max_semester_avg);
///     }
///
/// #    assert_eq!(Some(90f32), result);
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn define_sql_function(input: TokenStream) -> TokenStream {
    sql_function::expand(parse_macro_input!(input), false).into()
}

/// A legacy version of [`define_sql_function!`].
///
/// The difference is that it makes the helper type available in a module named the exact same as
/// the function:
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
/// This turned out to be an issue for the support of the `auto_type` feature, which is why
/// [`define_sql_function!`] was introduced (and why this is deprecated).
///
/// SQL functions declared with this version of the macro will not be usable with `#[auto_type]`
/// or `Selectable` `select_expression` type inference.
#[deprecated(since = "2.2.0", note = "Use [`define_sql_function`] instead")]
#[proc_macro]
#[cfg(all(feature = "with-deprecated", not(feature = "without-deprecated")))]
pub fn sql_function_proc(input: TokenStream) -> TokenStream {
    sql_function::expand(parse_macro_input!(input), true).into()
}

/// This is an internal diesel macro that
/// helps to implement all traits for tuples of
/// various sizes
#[doc(hidden)]
#[proc_macro]
pub fn __diesel_for_each_tuple(input: TokenStream) -> TokenStream {
    diesel_for_each_tuple::expand(parse_macro_input!(input)).into()
}

/// This is an internal diesel macro that
/// helps to restrict the visibility of an item based
/// on a feature flag
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __diesel_public_if(attrs: TokenStream, input: TokenStream) -> TokenStream {
    diesel_public_if::expand(parse_macro_input!(attrs), parse_macro_input!(input)).into()
}

/// Specifies that a table exists, and what columns it has. This will create a
/// new public module, with the same name, as the name of the table. In this
/// module, you will find a unit struct named `table`, and a unit struct with the
/// name of each column.
///
/// By default, this allows a maximum of 32 columns per table.
/// You can increase this limit to 64 by enabling the `64-column-tables` feature.
/// You can increase it to 128 by enabling the `128-column-tables` feature.
/// You can decrease it to 16 columns,
/// which improves compilation time,
/// by disabling the default features of Diesel.
/// Note that enabling 64 column tables or larger will substantially increase
/// the compile time of Diesel.
///
/// Example usage
/// -------------
///
/// ```rust
/// # extern crate diesel;
///
/// diesel::table! {
///     users {
///         id -> Integer,
///         name -> VarChar,
///         favorite_color -> Nullable<VarChar>,
///     }
/// }
/// ```
///
/// You may also specify a primary key if it is called something other than `id`.
/// Tables with no primary key aren't supported.
///
/// ```rust
/// # extern crate diesel;
///
/// diesel::table! {
///     users (non_standard_primary_key) {
///         non_standard_primary_key -> Integer,
///         name -> VarChar,
///         favorite_color -> Nullable<VarChar>,
///     }
/// }
/// ```
///
/// For tables with composite primary keys, list all the columns in the primary key.
///
/// ```rust
/// # extern crate diesel;
///
/// diesel::table! {
///     followings (user_id, post_id) {
///         user_id -> Integer,
///         post_id -> Integer,
///         favorited -> Bool,
///     }
/// }
/// # fn main() {
/// #     use diesel::prelude::Table;
/// #     use self::followings::dsl::*;
/// #     // Poor man's assert_eq! -- since this is type level this would fail
/// #     // to compile if the wrong primary key were generated
/// #     let (user_id {}, post_id {}) = followings.primary_key();
/// # }
/// ```
///
/// If you are using types that aren't from Diesel's core types, you can specify
/// which types to import.
///
/// ```
/// # extern crate diesel;
/// # mod diesel_full_text_search {
/// #     #[derive(diesel::sql_types::SqlType)]
/// #     pub struct TsVector;
/// # }
///
/// diesel::table! {
///     use diesel::sql_types::*;
/// #    use crate::diesel_full_text_search::*;
/// # /*
///     use diesel_full_text_search::*;
/// # */
///
///     posts {
///         id -> Integer,
///         title -> Text,
///         keywords -> TsVector,
///     }
/// }
/// # fn main() {}
/// ```
///
/// If you want to add documentation to the generated code, you can use the
/// following syntax:
///
/// ```
/// # extern crate diesel;
///
/// diesel::table! {
///     /// The table containing all blog posts
///     posts {
///         /// The post's unique id
///         id -> Integer,
///         /// The post's title
///         title -> Text,
///     }
/// }
/// ```
///
/// If you have a column with the same name as a Rust reserved keyword, you can use
/// the `sql_name` attribute like this:
///
/// ```
/// # extern crate diesel;
///
/// diesel::table! {
///     posts {
///         id -> Integer,
///         /// This column is named `mytype` but references the table `type` column.
///         #[sql_name = "type"]
///         mytype -> Text,
///     }
/// }
/// ```
///
/// This module will also contain several helper types:
///
/// dsl
/// ---
///
/// This simply re-exports the table, renamed to the same name as the module,
/// and each of the columns. This is useful to glob import when you're dealing
/// primarily with one table, to allow writing `users.filter(name.eq("Sean"))`
/// instead of `users::table.filter(users::name.eq("Sean"))`.
///
/// `all_columns`
/// -----------
///
/// A constant will be assigned called `all_columns`. This is what will be
/// selected if you don't otherwise specify a select clause. It's type will be
/// `table::AllColumns`. You can also get this value from the
/// `Table::all_columns` function.
///
/// star
/// ----
///
/// This will be the qualified "star" expression for this table (e.g.
/// `users.*`). Internally, we read columns by index, not by name, so this
/// column is not safe to read data out of, and it has had its SQL type set to
/// `()` to prevent accidentally using it as such. It is sometimes useful for
/// counting statements, however. It can also be accessed through the `Table.star()`
/// method.
///
/// `SqlType`
/// -------
///
/// A type alias called `SqlType` will be created. It will be the SQL type of
/// `all_columns`. The SQL type is needed for things like returning boxed
/// queries.
///
/// `BoxedQuery`
/// ----------
///
/// ```ignore
/// pub type BoxedQuery<'a, DB, ST = SqlType> = BoxedSelectStatement<'a, ST, table, DB>;
/// ```
#[proc_macro]
pub fn table_proc(input: TokenStream) -> TokenStream {
    match syn::parse(input) {
        Ok(input) => table::expand(input).into(),
        Err(_) => quote::quote! {
            compile_error!(
                "Invalid `table!` syntax. Please see the `table!` macro docs for more info.\n\
                 Docs available at: `https://docs.diesel.rs/master/diesel/macro.table.html`\n"
            );
        }
        .into(),
    }
}

/// This derives implements `diesel::Connection` and related traits for an enum of
/// connections to different databases.
///
/// By applying this derive to such an enum, you can use the enum as a connection type in
/// any location all the inner connections are valid. This derive supports enum
/// variants containing a single tuple field. Each tuple field type must implement
/// `diesel::Connection` and a number of related traits. Connection types form Diesel itself
/// as well as third party connection types are supported by this derive.
///
/// The implementation of `diesel::Connection::establish` tries to establish
/// a new connection with the given connection string in the order the connections
/// are specified in the enum. If one connection fails, it tries the next one and so on.
/// That means that as soon as more than one connection type accepts a certain connection
/// string the first matching type in your enum will always establish the connection. This
/// is especially important if one of the connection types is `diesel::SqliteConnection`
/// as this connection type accepts arbitrary paths. It should normally place as last entry
/// in your enum. If you want control of which connection type is created, just construct the
/// corresponding enum manually by first establishing the connection via the inner type and then
/// wrap the result into the enum.
///
/// # Example
/// ```
/// # extern crate diesel;
/// # use diesel::result::QueryResult;
/// use diesel::prelude::*;
///
/// #[derive(diesel::MultiConnection)]
/// pub enum AnyConnection {
/// #   #[cfg(feature = "postgres")]
///     Postgresql(diesel::PgConnection),
/// #   #[cfg(feature = "mysql")]
///     Mysql(diesel::MysqlConnection),
/// #   #[cfg(feature = "sqlite")]
///     Sqlite(diesel::SqliteConnection),
/// }
///
/// diesel::table! {
///     users {
///         id -> Integer,
///         name -> Text,
///     }
/// }
///
/// fn use_multi(conn: &mut AnyConnection) -> QueryResult<()> {
///    // Use the connection enum as any other connection type
///    // for inserting/updating/loading/…
///    diesel::insert_into(users::table)
///        .values(users::name.eq("Sean"))
///        .execute(conn)?;
///
///    let users = users::table.load::<(i32, String)>(conn)?;
///
///    // Match on the connection type to access
///    // the inner connection. This allows us then to use
///    // backend specific methods.
/// #    #[cfg(feature = "postgres")]
///    if let AnyConnection::Postgresql(ref mut conn) = conn {
///        // perform a postgresql specific query here
///        let users = users::table.load::<(i32, String)>(conn)?;
///    }
///
///    Ok(())
/// }
///
/// # fn main() {}
/// ```
///
/// # Limitations
///
/// The derived connection implementation can only cover the common subset of
/// all inner connection types. So, if one backend doesn't support certain SQL features,
/// like for example, returning clauses, the whole connection implementation doesn't
/// support this feature. In addition, only a limited set of SQL types is supported:
///
/// * `diesel::sql_types::SmallInt`
/// * `diesel::sql_types::Integer`
/// * `diesel::sql_types::BigInt`
/// * `diesel::sql_types::Double`
/// * `diesel::sql_types::Float`
/// * `diesel::sql_types::Text`
/// * `diesel::sql_types::Date`
/// * `diesel::sql_types::Time`
/// * `diesel::sql_types::Timestamp`
///
/// Support for additional types can be added by providing manual implementations of
/// `HasSqlType`, `FromSql` and `ToSql` for the corresponding type, all databases included
/// in your enum, and the backend generated by this derive called `MultiBackend`.
/// For example to support a custom enum `MyEnum` with the custom SQL type `MyInteger`:
/// ```
/// extern crate diesel;
/// use diesel::backend::Backend;
/// use diesel::deserialize::{self, FromSql, FromSqlRow};
/// use diesel::serialize::{self, IsNull, ToSql};
/// use diesel::AsExpression;
/// use diesel::sql_types::{HasSqlType, SqlType};
/// use diesel::prelude::*;
///
/// #[derive(diesel::MultiConnection)]
/// pub enum AnyConnection {
/// #   #[cfg(feature = "postgres")]
///     Postgresql(diesel::PgConnection),
/// #   #[cfg(feature = "mysql")]
///     Mysql(diesel::MysqlConnection),
/// #   #[cfg(feature = "sqlite")]
///     Sqlite(diesel::SqliteConnection),
/// }
///
/// // defining an custom SQL type is optional
/// // you can also use types from `diesel::sql_types`
/// #[derive(Copy, Clone, Debug, SqlType)]
/// #[diesel(postgres_type(name = "Int4"))]
/// #[diesel(mysql_type(name = "Long"))]
/// #[diesel(sqlite_type(name = "Integer"))]
/// struct MyInteger;
///
///
/// // our custom enum
/// #[repr(i32)]
/// #[derive(Debug, Clone, Copy, AsExpression, FromSqlRow)]
/// #[diesel(sql_type = MyInteger)]
/// pub enum MyEnum {
///     A = 1,
///     B = 2,
/// }
///
/// // The `MultiBackend` type is generated by `#[derive(diesel::MultiConnection)]`
/// // This part is only required if you define a custom sql type
/// impl HasSqlType<MyInteger> for MultiBackend {
///    fn metadata(lookup: &mut Self::MetadataLookup) -> Self::TypeMetadata {
///        // The `lookup_sql_type` function is exposed by the `MultiBackend` type
///        MultiBackend::lookup_sql_type::<MyInteger>(lookup)
///    }
/// }
///
/// impl FromSql<MyInteger, MultiBackend> for MyEnum {
///    fn from_sql(bytes: <MultiBackend as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
///        // The `from_sql` function is exposed by the `RawValue` type of the
///        // `MultiBackend` type
///        // This requires a `FromSql` impl for each backend
///        bytes.from_sql::<MyEnum, MyInteger>()
///    }
/// }
///
/// impl ToSql<MyInteger, MultiBackend> for MyEnum {
///    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, MultiBackend>) -> serialize::Result {
///        /// `set_value` expects a tuple consisting of the target SQL type
///        /// and self for `MultiBackend`
///        /// This requires a `ToSql` impl for each backend
///        out.set_value((MyInteger, self));
///        Ok(IsNull::No)
///    }
/// }
/// # #[cfg(feature = "postgres")]
/// # impl ToSql<MyInteger, diesel::pg::Pg> for MyEnum {
/// #    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, diesel::pg::Pg>) -> serialize::Result { todo!() }
/// # }
/// # #[cfg(feature = "mysql")]
/// # impl ToSql<MyInteger, diesel::mysql::Mysql> for MyEnum {
/// #    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, diesel::mysql::Mysql>) -> serialize::Result { todo!() }
/// # }
/// # #[cfg(feature = "sqlite")]
/// # impl ToSql<MyInteger, diesel::sqlite::Sqlite> for MyEnum {
/// #    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, diesel::sqlite::Sqlite>) -> serialize::Result { todo!() }
/// # }
/// # #[cfg(feature = "postgres")]
/// # impl FromSql<MyInteger, diesel::pg::Pg> for MyEnum {
/// #    fn from_sql(bytes: <diesel::pg::Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> { todo!() }
/// # }
/// # #[cfg(feature = "mysql")]
/// # impl FromSql<MyInteger, diesel::mysql::Mysql> for MyEnum {
/// #    fn from_sql(bytes: <diesel::mysql::Mysql as Backend>::RawValue<'_>) -> deserialize::Result<Self> { todo!() }
/// # }
/// # #[cfg(feature = "sqlite")]
/// # impl FromSql<MyInteger, diesel::sqlite::Sqlite> for MyEnum {
/// #    fn from_sql(bytes: <diesel::sqlite::Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> { todo!() }
/// # }
/// # fn main() {}
/// ```
#[proc_macro_derive(MultiConnection)]
pub fn derive_multiconnection(input: TokenStream) -> TokenStream {
    multiconnection::derive(syn::parse_macro_input!(input)).into()
}

/// Automatically annotates return type of a query fragment function
///
/// This may be useful when factoring out common query fragments into functions.
/// If not using this, it would typically involve explicitly writing the full
/// type of the query fragment function, which depending on the length of said
/// query fragment can be quite difficult (especially to maintain) and verbose.
///
/// # Example
///
/// ```rust
/// # extern crate diesel;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use schema::{users, posts};
/// use diesel::dsl;
///
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let conn = &mut establish_connection();
/// #
/// #[dsl::auto_type]
/// fn user_has_post() -> _ {
///     dsl::exists(posts::table.filter(posts::user_id.eq(users::id)))
/// }
///
/// let users_with_posts: Vec<String> = users::table
///     .filter(user_has_post())
///     .select(users::name)
///     .load(conn)?;
///
/// assert_eq!(
///     &["Sean", "Tess"] as &[_],
///     users_with_posts
///         .iter()
///         .map(|s| s.as_str())
///         .collect::<Vec<_>>()
/// );
/// #     Ok(())
/// # }
/// ```
/// # Limitations
///
/// While this attribute tries to support as much of diesels built-in DSL as possible it's
/// unfortunately not possible to support everything. Notable unsupported types are:
///
/// * Update statements
/// * Insert from select statements
/// * Query constructed by `diesel::sql_query`
/// * Expressions using `diesel::dsl::sql`
///
/// For these cases a manual type annotation is required. See the "Annotating Types" section below
/// for details.
///
///
/// # Advanced usage
///
/// By default, the macro will:
///  - Generate a type alias for the return type of the function, named the
///    exact same way as the function itself.
///  - Assume that functions, unless otherwise annotated, have a type alias for
///    their return type available at the same path as the function itself
///    (including case). (e.g. for the `dsl::not(x)` call, it expects that there
///    is a `dsl::not<X>` type alias available)
///  - Assume that methods, unless otherwise annotated, have a type alias
///    available as `diesel::dsl::PascalCaseOfMethodName` (e.g. for the
///    `x.and(y)` call, it expects that there is a `diesel::dsl::And<X, Y>` type
///    alias available)
///
/// The defaults can be changed by passing the following attributes to the
/// macro:
/// - `#[auto_type(no_type_alias)]` to disable the generation of the type alias.
/// - `#[auto_type(dsl_path = "path::to::dsl")]` to change the path where the
///     macro will look for type aliases for methods. This is required if you mix your own
///   custom query dsl extensions with diesel types. In that case, you may use this argument to
///   reference a module defined like so:
///   ```ignore
///   mod dsl {
///       /// export all of diesel dsl
///       pub use diesel::dsl::*;
///    
///       /// Export your extension types here
///       pub use crate::your_extension::dsl::YourType;
///    }
///    ```
/// - `#[auto_type(method_type_case = "snake_case")]` to change the case of the
///   method type alias.
/// - `#[auto_type(function_type_case = "snake_case")]` to change the case of
///   the function type alias (if you don't want the exact same path but want to
///   change the case of the last element of the path).
///
/// The `dsl_path` attribute in particular may be used to declare an
/// intermediate module where you would define the few additional needed type
/// aliases that can't be inferred automatically.
///
/// ## Annotating types
///
/// Sometimes the macro can't infer the type of a particular sub-expression. In
/// that case, you can annotate the type of the sub-expression:
///
/// ```rust
/// # extern crate diesel;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # use schema::{users, posts};
/// use diesel::dsl;
///
/// # fn main() {
/// #     run_test().unwrap();
/// # }
/// #
/// # fn run_test() -> QueryResult<()> {
/// #     let conn = &mut establish_connection();
/// #
/// // This will generate a `user_has_post_with_id_greater_than` type alias
/// #[dsl::auto_type]
/// fn user_has_post_with_id_greater_than(id_greater_than: i32) -> _ {
///     dsl::exists(
///         posts::table
///             .filter(posts::user_id.eq(users::id))
///             .filter(posts::id.gt(id_greater_than)),
///     )
/// }
///
/// #[dsl::auto_type]
/// fn users_with_posts_with_id_greater_than(id_greater_than: i32) -> _ {
///     // If we didn't specify the type for this query fragment, the macro would infer it as
///     // `user_has_post_with_id_greater_than<i32>`, which would be incorrect because there is
///     // no generic parameter.
///     let filter: user_has_post_with_id_greater_than =
///         user_has_post_with_id_greater_than(id_greater_than);
///     // The macro inferring that it has to pass generic parameters is still the convention
///     // because it's the most general case, as well as the common case within Diesel itself,
///     // and because annotating this way is reasonably simple, while the other way around
///     // would be hard.
///
///     users::table.filter(filter).select(users::name)
/// }
///
/// let users_with_posts: Vec<String> = users_with_posts_with_id_greater_than(2).load(conn)?;
///
/// assert_eq!(
///     &["Tess"] as &[_],
///     users_with_posts
///         .iter()
///         .map(|s| s.as_str())
///         .collect::<Vec<_>>()
/// );
/// #     Ok(())
/// # }
/// ```
#[proc_macro_attribute]
pub fn auto_type(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    dsl_auto_type::auto_type_proc_macro_attribute(
        proc_macro2::TokenStream::from(attr),
        proc_macro2::TokenStream::from(input),
        dsl_auto_type::DeriveSettings::builder()
            .default_dsl_path(parse_quote!(diesel::dsl))
            .default_generate_type_alias(true)
            .default_method_type_case(AUTO_TYPE_DEFAULT_METHOD_TYPE_CASE)
            .default_function_type_case(AUTO_TYPE_DEFAULT_FUNCTION_TYPE_CASE)
            .build(),
    )
    .into()
}

const AUTO_TYPE_DEFAULT_METHOD_TYPE_CASE: dsl_auto_type::Case = dsl_auto_type::Case::UpperCamel;
const AUTO_TYPE_DEFAULT_FUNCTION_TYPE_CASE: dsl_auto_type::Case = dsl_auto_type::Case::DoNotChange;
