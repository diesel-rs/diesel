#![recursion_limit = "1024"]
// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![allow(
    clippy::needless_pass_by_value,
    clippy::option_map_unwrap_or_else,
    clippy::option_map_unwrap_or
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
mod non_aggregate;
mod query_id;
mod queryable;
mod queryable_by_name;
mod sql_function;
mod sql_type;

use diagnostic_shim::*;

/// Implements `AsChangeset`
///
///  Structs which derive this trait must be annotated with
/// `#[table_name = "something"]`. If the field name of your struct differs
/// from the name of the column, you can annotate the field with
/// `#[column_name = "some_column_name"]`.
///
/// By default, any `Option` fields on the struct are skipped if their value is
/// `None`. If you would like to assign `NULL` to the field instead, you can
/// annotate your struct with `#[changeset_options(treat_none_as_null =
/// "true")]`.
///
/// # Attributes
///
/// ## Required type attributes
///
/// * `#[table_name = "some_table"]`, specifies the table for which the
/// current type is a changeset. Requires that `some_table` is in scope.
///
/// ## Optional type attributes
///
/// * `#[changeset_options(treat_none_as_null = "true")]`, specifies that
/// the derive should threat `None` values as `NULL`. By default
/// `Option::<T>::None` is just skipped. To insert a `NULL` using default
/// behavior use `Option::<Option<T>>::Some(None)`
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
/// you can specify this by adding the annotation `#[diesel(not_sized)]`.
/// This will skip the impls for non-reference types.
///
/// # Attributes:
///
/// ## Required type attributes
///
/// * `#[sql_type = "SqlType"]`, to specify the sql type of the
///  generated implementations. If the attribute exists multiple times
///  impls for each sql type are generated.
///
/// ## Optional type attribute
///
/// * `#[diesel(not_sized)]`, to skip generating impls that require
///   that the type is `Sized`
#[proc_macro_derive(AsExpression, attributes(diesel, sql_type))]
pub fn derive_as_expression(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, as_expression::derive)
}

/// Implement required traits for associations API
///
/// This derive implement support for diesels associations api. Check the
/// module level documentation for details.
///
/// # Attributes
///
/// # Required type attributes
///
/// * `#[belongs(User)]`, to specify a child-to-parent relation ship
/// between the current table and the specified parent type (`User`).
/// If this attribute is given multiple times, multiple relation ships
/// are generated.
/// * `#[belongs_to(User, foreign_key = "mykey")]`, variant of the attribute
/// above. Allows to specify the name of the foreign key. If the foreign key
/// is not specified explicitly, the remote lower case type name with an
/// appended `_id` is used as foreign key name. (`user_id` in this example
/// case)
///
/// # Optional type attributes
///
/// * `#[table_name = "some_table_name"]` specifies the table this
///    type belongs to. Requires that `some_table_name` is in scope.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name
///
/// # Optional field attributes
///
/// * `#[column_name = "some_table_name"]`, overrides the column the current
/// field maps to to `some_table_name`. By default the field name is used
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

/// Implements `FromSqlRow` and `Queryable`
///
/// This derive is mostly useful to implement support deserializing
/// into rust types not supported by diesel itself.
///
/// There are no options or special considerations needed for this derive.
#[proc_macro_derive(FromSqlRow, attributes(diesel))]
pub fn derive_from_sql_row(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, from_sql_row::derive)
}

/// Implements `Identifiable`
///
/// By default, the "id" field is assumed to be a single field called `id`.
/// If it's not, you can put `#[primary_key(your_id)]` on your struct.
/// If you have a composite primary key, the syntax is `#[primary_key(id1, id2)]`.
///
/// By default, `#[derive(Identifiable)]` will assume that your table
/// name is the plural form of your struct name.
/// Diesel uses very simple pluralization rules.
/// It only adds an `s` to the end, and converts `CamelCase` to `snake_case`.
/// If your table name does not follow this convention
/// or the plural form isn't just an `s`,
/// you can specify the table name with `#[table_name = "some_table_name"]`.
/// Our rules for inferring table names is considered public API.
/// It will never change without a major version bump.
///
/// # Attributes
///
/// ## Optional type attributes
///
/// * `#[table_name = "some_table_name"]` specifies the table this
///    type belongs to. Requires that `some_table_name` is in scope.
///    If this attribute is not used, the type name converted to
///    `snake_case` with an added `s` is used as table name
/// * `#[primary_key(id1, id2)]` to specify the struct field that
///    that corresponds to the primary key. If not used, `id` will be
///    assumed as primary key field
///
///
#[proc_macro_derive(Identifiable, attributes(table_name, primary_key, column_name))]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, identifiable::derive)
}

/// Implements `Insertable`
///
/// Structs which derive this trait must also be annotated
/// with `#[table_name = "some_table_name"]`. If the field name of your
/// struct differs from the name of the column, you can annotate the field
/// with `#[column_name = "some_column_name"]`.
///
/// Your struct can also contain fields which implement `Insertable`. This is
/// useful when you want to have one field map to more than one column (for
/// example, an enum that maps to a label and a value column). Add
/// `#[diesel(embed)]` to any such fields.
///
/// # Attributes
///
/// ## Required type attributes
///
/// * `#[table_name = "some_table_name"]`, specifies the table this type
/// is insertable into. Requires that `some_table_name` is in scope.
///
/// ## Optional field attributes
///
/// * `#[column_name = "some_table_name"]`, overrides the column the current
/// field maps to `some_table_name`. By default the field name is used
/// as column name
/// * `#[diesel(embed)]`, specifies that the current field maps not only
/// to single database field, but is a struct that implements `Insertable`
#[proc_macro_derive(Insertable, attributes(table_name, column_name, diesel))]
pub fn derive_insertable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, insertable::derive)
}

/// Implements `NonAggregate`
///
/// This derive can be used for structs with no type parameters,
/// which are not aggregate, as well as for struct which are
/// `NonAggregate` if all type parameters are `NonAggregate`.
#[proc_macro_derive(NonAggregate)]
pub fn derive_non_aggregate(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, non_aggregate::derive)
}

/// Implements `QueryId`
#[proc_macro_derive(QueryId)]
pub fn derive_query_id(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, query_id::derive)
}

/// Implements `Queryable`
///
/// This trait can only be derived for structs, not enums.
///
/// **When this trait is derived, it will assume that the order of fields on your
/// struct match the order of the fields in the query. This means that field
/// order is significant if you are using `#[derive(Queryable)]`. Field name has
/// no effect.**
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = "Type")]`. If this attribute is present, Diesel
/// will deserialize into that type, rather than the type on your struct and
/// call `.into` to convert it. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
///
/// # Attributes
///
/// ## Optional field attributes:
///
/// * `#[diesel(deserialize_as = "Type")]`, instead of deserializing directly
///   into the field type, the implementation will deserialize into `Type`.
///   Then `Type` is converted via `.into()` into the field type. By default
///   this derive will deserialize directly into the field type
#[proc_macro_derive(Queryable, attributes(column_name, diesel))]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, queryable::derive)
}

/// Implements `QueryableByName`
///
/// To derive this trait, Diesel needs to know the SQL type of each field. You
/// can do this by either annotating your struct with `#[table_name =
/// "some_table"]` (in which case the SQL type will be
/// `diesel::dsl::SqlTypeOf<table_name::column_name>`), or by annotating each
/// field with `#[sql_type = "SomeType"]`.
///
/// If you are using `#[table_name]`, the module for that table must be in
/// scope. For example, to derive this for a struct called `User`, you will
/// likely need a line such as `use schema::users;`
///
/// If the name of a field on your struct is different than the column in your
/// `table!` declaration, or if you are deriving this trait on a tuple struct,
/// you can annotate the field with `#[column_name = "some_column"]`. For tuple
/// structs, all fields must have this annotation.
///
/// If a field is another struct which implements `QueryableByName`, instead of
/// a column, you can annotate that struct with `#[diesel(embed)]`
///
/// To provide custom deserialization behavior for a field, you can use
/// `#[diesel(deserialize_as = "Type")]`. If this attribute is present, Diesel
/// will deserialize into that type, rather than the type on your struct and
/// call `.into` to convert it. This can be used to add custom behavior for a
/// single field, or use types that are otherwise unsupported by Diesel.
///
/// # Attributes
///
/// ## Type attributes
///
/// * `#[table_name = "some_table"]`, to specify that this type contains
///   columns for the specified table. If no field attributes are specified
///   the derive will use the sql type of the corresponding column.
///
/// ## Field attributes
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
#[proc_macro_derive(QueryableByName, attributes(table_name, column_name, sql_type, diesel))]
pub fn derive_queryable_by_name(input: TokenStream) -> TokenStream {
    expand_proc_macro(input, queryable_by_name::derive)
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
/// For PostgreSQL, add `#[postgres(oid = "some_oid", array_oid = "some_oid")]`
/// or `#[postgres(type_name = "pg_type_name")]` if the OID is not stable.
/// For MySQL, specify which variant of `MysqlType` should be used
/// by adding `#[mysql_type = "Variant"]`.
/// For SQLite, specify which variant of `SqliteType` should be used
/// by adding `#[sqlite_type = "Variant"]`.
///
/// # Attributes
///
/// ## Type attributes
///
/// * `#[postgres(type_name = "TypeName")]` specifies support for
/// a postgresql type with the name `TypeName`. Prefer this variant
/// for types with no stable OID (== everything but the builtin types)
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
