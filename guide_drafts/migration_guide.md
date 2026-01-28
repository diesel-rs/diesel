# Diesel 2.0 Migration guide

Diesel 2.0 introduces substantial changes to Diesel's inner workings. 
In some cases this impacts code written using Diesel 1.4.x. 
This document outlines notable changes and presents potential update strategies. 
We recommend to start the upgrade by removing the usage of all items that 
are marked as deprecated in Diesel 1.4.x.

Any code base using migrating to Diesel 2.0 is expected to be affected at least by 
the following changes:

* [Diesel now requires a mutable reference to the connection](#2-0-0-mutable-connection)
* [Changed derive attributes](#2-0-0-derive-attributes)

Users of `diesel_migration` are additionally affected by the following change:

* [`diesel_migration` rewrite](#2-0-0-upgrade-migrations)

Users of `BoxableExpression` might be affected by the following change:

* [Changed nullability of operators](#2-0-0-nullability-ops)

Users of tables containing a column of the type `Array<T>` are affected by the following change:

* [Changed nullability of array elements](#2-0-0-nullability-of-array-elements)

Users that implement support for their SQL types or type mappings are affected 
by the following changes:

* [Changed required traits for custom SQL types](#2-0-0-custom-type-implementation)
* [Changed `ToSql` implementations](#2-0-0-to-sql)
* [Changed `FromSql` implementations](#2-0-0-from-sql)

`no_arg_sql_function!` macro is now pending deprecation.
Users of the macro are advised to consider `define_sql_function!` macro.

* [Deprecated usage of `no_arg_sql_function!` macro](#2-0-0-no_arg_sql_function)

Users that update generic Diesel code will also be affected by the following changes:

* [Removing `NonAggregate` in favor of `ValidGrouping`](#2-0-0-upgrade-non-aggregate)
* [Changed generic bounds](#2-0-0-generic-changes)

Additionally this release contains many changes for users that implemented a custom backend/connection.
We do not provide explicit migration steps but we encourage users to reach out with questions pertaining to these changes. 


## Mutable Connections required<a name="2-0-0-mutable-connection"></a>

Diesel now requires mutable access to the `Connection` to perform any database interaction. The following changes
are required for all usages of any `Connection` type:

```diff
- let connection = PgConnection::establish_connection("…")?;
- let result = some_query.load(&connection)?;
+ let mut connection = PgConnection::establish_connection("…")?;
+ let result = some_query.load(&mut connection)?;
```

We expect this to be a straightforward change as the connection already can execute only one query at a time.


## Derive attributes<a name="2-0-0-derive-attributes"></a>

We have updated all of our Diesel derive attributes to follow the patterns that are used
widely in the Rust's ecosystem. This means that all of them need to be wrapped by `#[diesel()]` now.  You can now specify multiple attributes on the same line using `,` separator.

This is backward compatible and thus all of your old attributes will still work, but with
warnings. The attributes can be upgraded by either looking at the warnings or by reading
diesel derive documentation reference.

## `diesel_migration` rewrite<a name = "2-0-0-upgrade-migrations"></a>

We have completely rewritten the `diesel_migration` crate. As a part of this rewrite all 
free standing functions are removed from `diesel_migration`. Equivalent functionality 
is now provided by the `MigrationHarness` trait, which is implemented for any `Connection` 
type and for `HarnessWithOutput`. Refer to their documentations for details.

Additionally, this rewrite changed the way we provide migrations. Instead of having our own implementation
for file based and embedded migration we now provide a unified `MigrationSource` trait to abstract 
over the differences. `diesel_migration` provides two implementations:

* `FileBasedMigrations`, which mirrors the existing behaviour to load raw sql migrations at run time
form a specific directory
* `EmbeddedMigrations`, which mirrors the existing `embed_migrations!` macro. 

Finally the `embed_migrations!()` macro itself changed. Instead of generating a magical embedded module 
it now generates an instance of `EmbeddedMigrations`, that could be stored in a constant for example.

That means code using `embed_migrations!()` needs to be changed from
```rust
embed_migrations!()

fn run_migration(conn: &PgConnection) {
    embedded_migrations::run(conn).unwrap()
}
```
to 
```rust
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn run_migration(conn: &PgConnection) {
    conn.run_pending_migrations(MIGRATIONS).unwrap();
}
```

## Changed nullability of operators<a name="2-0-0-nullability-ops"></a>

We changed the way how we handle the propagation of null values through binary operators. Diesel 1.x always assumed 
that the result of a binary operation `value_a > value_b` is not nullable, which does not match the behaviour of the 
underlying databases. `value_a > null` may return a `NULL` value there. With Diesel 2.0 we changed this to match more
closely the behaviour of the underlying databases. We expect this change to have the biggest impact on existing usages
of `BoxableExpression` as it may change the resulting sql type there. As a possible workaround for divering sql types 
there we recommend to use one of the following functions:

* `NullableExpressionMethods::nullable()`
* `NullableExpressionMethods::assume_not_null()`

## Changed nullability of array elements<a name="#2-0-0-nullability-of-array-elements"></a>

We changed the inferred SQL type for columns with array types for the PostgreSQL backend. Instead of using `Array<ST>` 
we now infer `Array<Nullable<ST>>` to support arrays containing `NULL` values. This change implies a change mapping
of columns of the corresponding types. It is possible to handle this change using one of the following strategies:

* Use `Vec<Option<T>>` as rust side type instead of `Vec<T>`
* Manually set the corresponding column to `Array<ST>` in your schema, to signal that this array does not contain null values. You may want to use the `patch_file` key for diesel CLI for this.
* Use `#[diesel(deserialize_as = "…")]` to explicitly overwrite the  deserialization implementation used for this specific struct field. Checkout the documentation of `#[derive(Queryable)]` for details.

## Custom SQL type implementations<a name="2-0-0-custom-type-implementation"></a>

We changed how we mark sql types as nullable at type level. For this we replaced the `NonNull` trait with a 
more generic `SqlType` trait, which allows to mark a sql type as (non-) nullable. This may affect custom
sql type implementations.

Users that already use the existing `#[derive(SqlType)]` do not need to change any code. The derive internally
generates the correct code after the update. Users that use a manual implementation of `NonNull` need to replace
it with a corresponding `SqlType` implementation:

```diff
- impl NonNull for MyCustomSqlType {}
+ impl SqlType for MyCustomSqlType {
+     type IsNull = diesel::sql_types::is_nullable::NotNull;
+ }
```

Additionally, the diesel CLI tool was changed so that it automatically generates the Rust side definition of custom SQL types 
as long as they appear on any table. This feature currently only supports the PostgreSQL backend, as all other supported backends
do not support real custom types at SQL level at all.

## Changed `ToSql` implementations<a name="2-0-0-to-sql"></a>

We restructured the way Diesel serializes Rust values to their backend specific representation.
This enables us to skip copying the value at all if the specific backend supports writing to a 
shared buffer. Unfortunately, this feature requires changes to the `ToSql` trait. This change introduces
a lifetime that ensures that a value implementing `ToSql` outlives the underlying serialisation buffer.
Additionally we separated the output buffer type for Sqlite from the type used for PostgreSQL and Mysql.

This has the implication that for generic implementations using a inner existing `ToSql` implementation you cannot
create temporary values anymore and forward them to the inner implementation.

For backend concrete implementations, the following functions allow You to work around this limitation:

* `Output::reborrow()` for the `Pg` and `Mysql` backend
* `Output::set_value()` for the `Sqlite` backend (Refer to the documentation of `SqliteBindValue` for accepted values)



## Changed `FromSql` implementations<a name="2-0-0-from-sql"></a>

We changed the raw value representation for both PostgreSQL and MySQL
backends, from a `& [u8]` to an opaque type. This allows us to include additional information like the database side
type there. This change enables users to write `FromSql` implementations that decide dynamically what kind of value
was received. The new value types for both backends expose a `as_bytes()` method to access the underlying byte buffer.

Any affected backend needs to perform the following changes:

```diff
impl<DB: Backend> FromSql<YourSqlType, DB> for YourType {
-    fn from_sql(bytes: &[u8]) -> deserialize::Result<Self> {
+    fn from_sql(value: backend::RawValue<'_, DB>) -> deserialize::Result<Self> {
+        let bytes = value.as_bytes();
         // …
     }
}
```



## `no_arg_sql_function`<a name="2-0-0-no_arg_sql_function"></a>

The `no_arg_sql_function` was deprecated without direct replacement. However the
`define_sql_function!` macro gained support for sql functions without argument. This support generates slightly 
different code. Instead of representing the sql function as zero sized struct, `define_sql_function!` will generate an ordinary function call without arguments. This requires changing any usage of the generated dsl. This change 
affects all of the usages of the `no_arg_sql_function!` in third party crates.

```diff
- no_arg_sql_function!(now, sql_types::Timestamp, "Represents the SQL NOW() function");
- 
- diesel::select(now)
 
+ define_sql_function!{
+     /// Represents the SQL NOW() function
+     fn now() -> sql_types::Timestamp;
+ }
+
+ diesel::select(now())
```

### Replacement of `NonAggregate` with `ValidGrouping`<a name="2-0-0-upgrade-non-aggregate"></a>

Diesel now fully enforces the aggregation rules, which required us to change the way we represent the aggregation 
at the type system level. This is used to provide `group_by` support. Diesel's aggregation rules 
match the semantics of PostgreSQL or MySQL with the `ONLY_FULL_GROUP_BY` option enabled.

As part of this change we removed the `NonAggregate` trait in favor of a new, more expressive `ValidGrouping`
trait. Existing implementations of `NonAggregate` must be replaced with an equivalent `ValidGrouping` implementation.

The following change shows how to replace an existing implementation with a strictly equivalent implementation.
```diff
- impl NonAggregate for MyQueryNode {}
+ impl ValidGrouping<()> for MyQueryNode {
+    type IsAggregate = is_aggregate::No;
+ }
```
Additional changes may be required to adapt custom query ast implementations to fully support `group_by` clauses. 
Refer to the documentation of `ValidGrouping` for details.

In addition, any occurrence of `NonAggregate` in trait bounds needs to be replaced. Again, the following
change shows the strictly equivalent version:

```diff
 where
-     T: NonAggregate,
+     T: ValidGrouping<()>,
+     T::IsAggregate: MixedGrouping<is_aggregate::No, Output = is_aggregate::No>,
+     is_aggregate::No: MixedGrouping<T::IsAggregate, Output = is_aggregate::No>,
```


## Other changes to generics<a name="2-0-0-generic-changes">

In addition to the changes listed above, we changed numerous internal details of Diesel. This will have impact on
most codebases that include non-trivial generic code abstracting over Diesel. This section tries to list as much of those
changes as possible

### Removed most of the non-public reachable API

With Diesel 2.0 we removed most of the API which was marked with `#[doc(hidden)]`. Technically these parts of the API 
have always been private to Diesel. This change enforces this distinction in stricter way. In addition, some
parts of these formerly hidden API are now documented and exposed behind the
`i-implement-a-third-party-backend-and-opt-into-breaking-changes` crate feature. As the name already implies 
we reserve the right to change these APIs between different Diesel 2.x minor releases, so you should always pin
a concrete minor release version if you use these APIs.
If you depended on such an API and you cannot find a suitable replacement we invite you to work with us on exposing the corresponding 
feature as part of the stable API.

### Changed structure of the deserialization traits

We changed the internal structure of the `FromSqlRow`, `Queryable` and `QueryableByName` trait family used for deserialization. This change allows us to unify our deserialization code.
We hopefully put sufficient wild card implementations in place so that old trait bounds imply 
the right trait anyway. For cases where this does not hold true, the following changes may be required:

`Queryable<ST, DB>` is now equivalent to `FromSqlRow<ST, DB>`. The latter is used as an actual trait bound 
on the corresponding `RunQueryDsl` methods.
`QueryableByName<DB>` is now equivalent to `FromSqlRow<Untyped, DB>`. The latter is used as an actual trait 
on the corresponding `RunQueryDsl` methods.

### Changed the scope of `QueryFragment` implementations

With Diesel 2.0, we introduced a way to specialise `QueryFragment` implementations for specific backend, while 
providing a generic implementation for other backends. To be able to use this feature in the future we marked
existing wild card `QueryFragment` implementations with an additional `DieselReserveSpecialization`. 
Rustc suggests just adding an additional trait bound on this trait. It's not possible to add a bound on 
this trait without opting into breaking changes and it's almost never required to actually do that. Any
occurrence of an error mentioning this trait can simply be fixed by adding a trait bound like follows:
```rust
where
    QueryAstNodeMentionedInTheErrorMessage: QueryFragment<BackendType>
```

This rule has one notable exception: Third party backend implementations. We expect those backends to opt into the 
`i-implement-a-third-party-backend-and-opt-into-breaking-changes` feature anyway, as it's otherwise not possible to 
implement a third party backend.
