# Change Log
All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## Unreleased

### Added

* The `Insertable!` macro can now be used instead of `#[insertable_into]` for
  those wishing to avoid syntax extensions from `diesel_codegen`. See
  http://docs.diesel.rs/diesel/macro.Insertable!.html for details.

* The `Queryable!` macro can now be used instead of `#[derive(Queryable)]` for
  those wishing to avoid syntax extensions from `diesel_codegen`. See
  http://docs.diesel.rs/diesel/macro.Queryable!.html for details.

* The `Identifiable!` macro can now be used instead of `#[derive(Identifiable)]` for
  those wishing to avoid syntax extensions from `diesel_codegen`. See
  http://docs.diesel.rs/diesel/macro.Identifiable!.html for details.

* The `AsChangeset!` macro can now be used instead of `#[changeset_for(table)]`
  for those wishing to avoid syntax extensions from `diesel_codegen`. See
  http://docs.diesel.rs/diesel/macro.AsChangeset!.html for details.

* Added support for the PostgreSQL `ALL` operator. See
  http://docs.diesel.rs/diesel/pg/expression/dsl/fn.all.html for details.

### Changed

* Diesel now targets `nightly-2016-07-07`. Future releases will update to a
  newer nightly version on the date that Rust releases.

* Most structs that implement `Queryable` will now also need
  `#[derive(Identifiable)]`.

* `infer_schema!` on SQLite now accepts a larger range of type names

* `types::VarChar` is now an alias for `types::Text`. Most code should be
  unaffected by this. PG array columns are treated slightly differently,
  however. If you are using `varchar[]`, you should switch to `text[]` instead.

* Struct fields annotated with `#[column_name="name"]` should be changed to
  `#[column_name(name)]`.

* The structure of `DatabaseError` has changed to hold more information. See
  http://docs.diesel.rs/diesel/result/enum.Error.html and
  http://docs.diesel.rs/diesel/result/trait.DatabaseErrorInformation.html for
  more information

* Structs which implement `Identifiable` can now be passed to `update` and
  `delete`. This means you can now write `delete(&user).execute(&connection)`
  instead of `delete(users.find(user.id)).execute(&connection)`

### Fixed

* `&&[T]` can now be used in queries. This allows using slices with things like
  `#[insertable_into]`.

## [0.6.1] 2016-04-14

### Added

* Added the `escape` method to `Like` and `NotLike`, to specify the escape
  character used in the pattern. See [EscapeExpressionMethods][escape] for
  details.

[escape]: http://docs.diesel.rs/diesel/expression/expression_methods/escape_expression_methods/trait.EscapeExpressionMethods.html

### Fixed

* `diesel_codegen` and `diesel_cli` now properly rely on Diesel 0.6.0. The
  restriction to 0.5.0 was an oversight.

* `infer_schema!` now properly excludes metadata tables on SQLite.

* `infer_schema!` now properly maps types on SQLite.

## [0.6.0] 2016-04-12

### Added

* Queries can now be boxed using the `into_boxed()` method. This is useful for
  conditionally modifying queries without changing the type. See
  [BoxedDsl][boxed_dsl] for more details.

* `infer_schema!` is now supported for use with SQLite3.

* The maximum table size can be increased to 52 by enabling the `huge-tables`
  feature. This feature will substantially increase compile times.

* The `DISTINCT` keyword can now be added to queries via the `distinct()`
  method.

* `SqliteConnection` now implements `Send`

[boxed_dsl]: http://docs.diesel.rs/diesel/prelude/trait.BoxedDsl.html

### Changed

* `diesel::result::Error` now implements `Send` and `Sync`. This required a
  change in the return type of `ToSql` and `FromSql` to have those bounds as
  well.

* It is no longer possible to pass an owned value to `diesel::insert`. `insert`
  will now give a more helpful error message when you accidentally try to pass
  an owned value instead of a reference.

### Fixed

* `#[insertable_into]` can now be used with structs that have lifetimes with
  names other than `'a'`.

* Tables with a single column now properly return a single element tuple. E.g.
  if the column was of type integer, then `users::all_columns` is now `(id,)`
  and not `id`.

* `infer_schema!` can now work with tables that have a primary key other than
  `id`.

### Removed

* Removed the `no select` option for the `table!` macro. This was a niche
  feature that didn't fit with Diesel's philosophies. You can write a function
  that calls `select` for you if you need this functionality.

## [0.5.4] 2016-03-23

* Updated `diesel_codegen` to allow syntex versions up to 0.30.0.

## [0.5.3] 2016-03-12

### Added

* Added helper function `diesel_manage_updated_at('TABLE_NAME')` to postgres
  upon database setup. This function sets up a trigger on the specified table
  that automatically updates the `updated_at` column to the `current_timestamp`
  for each affected row in `UPDATE` statements.

* Added support for explicit `RETURNING` expressions in `INSERT` and `UPDATE`
  queries. Implicitly these queries will still use `RETURNING *`.

### Fixed

* Updated to work on nightly from early March

## [0.5.2] 2016-02-27

* Updated to work on nightly from late February

## [0.5.1] 2016-02-11

* Diesel CLI no longer has a hard dependency on SQLite and PostgreSQL. It
  assumes both by default, but if you need to install on a system that doesn't
  have one or the other, you can install it with `cargo install diesel_cli
  --no-default-features --features postgres` or `cargo install diesel_cli
  --no-default-features --features sqlite`

## [0.5.0] 2016-02-05

### Added

* Added support for SQLite. Diesel still uses postgres by default. To use SQLite
  instead, add `default-features = false, features = ["sqlite"]` to your
  Cargo.toml. You'll also want to add `default-features = false, features =
  ["sqlite"]` to `diesel_codegen`.
  Since SQLite is a much more limited database, it does not support our full set
  of features. You can use SQLite and PostgreSQL in the same project if you
  desire.

* Added support for mapping `types::Timestamp`, `types::Date`, and `types::Time`
  to/from `chrono::NaiveDateTime`, `chrono::NaiveDate`, and `chrono::NaiveTime`.
  Add `features = ["chrono"]` to enable.

* Added a `treat_none_as_null` option to `changeset_for`. When set to `true`,
  a model will set a field to `Null` when an optional struct field is `None`,
  instead of skipping the field entirely. The default value of the option is
  `false`, as we think the current behavior is a much more common use case.

* Added `Expression#nullable()`, to allow comparisons of not null columns with
  nullable ones when required.

* Added `sum` and `avg` functions.

* Added the `diesel setup`, `diesel database setup`, and `diesel database
  reset` commands to the CLI.

* Added support for SQL `IN` statements through the `eq_any` method.

* Added a top level `select` function for select statements with no from clause.
  This is primarily intended to be used for testing Diesel itself, but it has
  been added to the public API as it will likely be useful for third party
  crates in the future. `select(foo).from(bar)` might be a supported API in the
  future as an alternative to `bar.select(foo)`.

* Added `expression::dsl::sql` as a helper function for constructing
  `SqlLiteral` nodes. This is primarily intended to be used for testing Diesel
  itself, but is part of the public API as an escape hatch if our query builder
  DSL proves inadequate for a specific case. Use of this function in any
  production code is discouraged as it is inherently unsafe and avoids real type
  checking.

### Changed

* Moved most of our top level trait exports into a prelude module, and
  re-exported our CRUD functions from the top level.
  `diesel::query_builder::update` and friends are now `diesel::update`, and you
  will get them by default if you import `diesel::*`. For a less aggressive
  glob, you can import `diesel::prelude::*`, which will only export our traits.

* `Connection` is now a trait instead of a struct. The struct that was
  previously known as `Connection` can be found at `diesel::pg::PgConnection`.

* Rename both the `#[derive(Queriable)]` attribute and the `Queriable` trait to
  use the correct spelling `Queryable`.

* `load` and `get_results` now return a `Vec<Model>` instead of an iterator.

* Replaced `Connection#find(source, id)` with
  `source.find(id).first(&connection)`.

* The `debug_sql!` macro now uses \` for identifier quoting, and `?` for bind
  parameters, which is closer to a "generic" backend. The previous behavior had
  no identifier quoting, and used PG specific bind params.

* Many user facing types are now generic over the backend. This includes, but is
  not limited to `Queryable` and `Changeset`. This change should not have much
  impact, as most impls will have been generated by diesel_codegen, and that API
  has not changed.

* The mostly internal `NativeSqlType` has been removed. It now requires a known
  backend. `fn<T> foo() where T: NativeSqlType` is now `fn<T, DB> foo() where
  DB: HasSqlType<T>`

### Removed

* `Connection#query_sql` and `Connection#query_sql_params` have been removed.
  These methods were not part of the public API, and were only meant to be used
  for testing Diesel itself. However, they were technically callable from any
  crate, so the removal has been noted here. Their usage can be replaced with
  bare `select` and `expression::dsl::sql`.

## [0.4.1] 2016-01-11

### Changed

* Diesel CLI will no longer output notices about `__diesel_schema_migrations`
  already existing.

* Relicensed under MIT/Apache dual

## [0.4.0] 2016-01-08

### Added

* Added Diesel CLI, a tool for managing your schema.
  See [the readme](https://github.com/diesel-rs/diesel/blob/v0.4.0/README.md#database-migrations)
  for more information.

* Add the ability for diesel to maintain your schema for you automatically. See
  the [migrations](http://docs.diesel.rs/diesel/migrations/index.html)
  module for individual methods.

* Add DebugQueryBuilder to build sql without requiring a connection.

* Add print_sql! and debug_sql! macros to print out and return sql strings from
  QueryFragments.

### Fixed

* `#[changeset_for]` can now be used with structs containing a `Vec`. Fixes
  [#63](https://github.com/diesel-rs/diesel/issues/63).

* No longer generate invalid SQL when an optional update field is not the first
  field on a changeset. Fixes [#68](https://github.com/diesel-rs/diesel/issues/68).

* `#[changeset_for]` can now be used with structs containing only a single field
  other than `id`. Fixes [#66](https://github.com/diesel-rs/diesel/issues/66).

* `infer_schema!` properly works with array columns. Fixes
  [#65](https://github.com/diesel-rs/diesel/issues/65).

## [0.3.0] 2015-12-04

### Changed

* `#[changeset_for(table)]` now treats `Option` fields as an optional update.
  Previously a field with `None` for the value would insert `NULL` into the
  database field. It now does not update the field if the value is `None`.

* `.save_changes` (generated by `#[changeset_for]`) now returns a new struct,
  rather than mutating `self`. The returned struct can be any type that
  implements `Queryable` for the right SQL type

### Fixed

* `#[derive(Queryable)]` now allows generic parameters on the struct.

* Table definitions can now support up to 26 columns. Because this increases our
  compile time by 3x, `features = ["large-tables"]` is needed to support table
  definitions above 16 columns.

### Added

* Quickcheck is now an optional dependency. When `features = ["quickcheck"]` is
  added to `Cargo.toml`, you'll gain `Arbitrary` implementations for everything
  in `diesel::data_types`.

* Added support for the SQL `MIN` function.

* Added support for the `Numeric` data type. Since there is no Big Decimal type
  in the standard library, a dumb struct has been provided which mirrors what
  Postgres provides, which can be converted into whatever crate you are using.

* Timestamp columns can now be used with `std::time::SystemTime` when compiled
  with `--features unstable`

* Implemented `Send` on `Connection` (required for R2D2 support)

* Added `infer_schema!` and `infer_table_from_schema!`. Both macros take a
  database URL, and will invoke `table!` for you automatically based on the
  schema. `infer_schema!` queries for the table names, while
  `infer_table_from_schema!` takes a table name as the second argument.

## [0.2.0] - 2015-11-30

### Added

* Added an `execute` method to `QueryFragment`, which is intended to replace
  `Connection#execute_returning_count`. The old method still exists for use
  under the hood, but has been hidden from docs and is not considered public
  API.

* Added `get_result` and `get_results`, which work similarly to `load` and
  `first`, but are intended to make code read better when working with commands
  like `create` and `update`. In the future, `get_result` may also check that
  only a single row was affected.

* Added [`insert`][insert], which mirrors the pattern of `update` and `delete`.

### Changed

* Added a hidden `__Nonexhaustive` variant to `result::Error`. This is not
  intended to be something you can exhaustively match on, but I do want people
  to be able to check for specific cases, so `Box<std::error::Error>` is
  not an option.

* `query_one`, `find`, and `first` now assume a single row is returned. For
  cases where you actually expect 0 or 1 rows to be returned, the `optional`
  method has been added to the result, in case having a `Result<Option<T>>` is
  more ideomatic than checking for `Err(NotFound)`.

### Deprecated

* `Connection#insert` and `Connection#insert_returning_count` have been
  deprecated in favor of [`insert`][insert]

[insert]: http://docs.diesel.rs/diesel/query_builder/fn.insert.html

## [0.1.0] - 2015-11-29

* Initial release
