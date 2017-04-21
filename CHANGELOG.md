# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## Unreleased


### Added

* Added the `migration list` command to Diesel CLI for listing all available migrations and marking those that have been applied.

* Added support for adding two nullable columns.

* Addded support for unsigned types in MySQL.


## [0.12.0] - 2017-03-16

### Added

* Added support for the majority of PG upsert (`INSERT ON CONFLICT`). We now
  support specifying the constraint, as well as `DO UPDATE` in addition to `DO
  NOTHING`. See [the module docs][upsert-0.12.0] for details.

[upsert-0.12.0]: http://docs.diesel.rs/diesel/pg/upsert/index.html

* Added support for the SQL concatenation operator `||`. See [the docs for
  `.concat`][concat-0.12.0] for more details.

[concat-0.12.0]: http://docs.diesel.rs/diesel/expression/expression_methods/text_expression_methods/trait.TextExpressionMethods.html#method.concat

* Added support for the PostgreSQL [`Money` type][pg-money-0.12.0].

[pg-money-0.12.0]: https://www.postgresql.org/docs/9.6/static/datatype-money.html

* Diesel CLI: Added `db` as an alias for `database`, so you can now write `diesel db setup` (which is almost 40% faster!).

* The `table!` macro now allows you to use types from crates outside of Diesel.
  You can specify where types should be imported from by doing: `table! { use
  some_modules::*; foo { columns... }`. Not specifying any any modules is
  equivalent to `use diesel::types::*;`.

### Fixed

* `diesel_codegen` will provide a more useful error message when it encounters
  an unsupported type that contains a space in MySQL.

* `#[derive(AsChangeset)]` will now respect custom `#[primary_key]` annotations,
  and avoid setting those columns.

### Removed

* `WithDsl` and `Aliased` have been removed. They were a feature that was
  actually closer to a cross join than the names implied, and wasn't fully
  thought out. The functionality they provided will return as joins are further
  revamped.

* The internal use macro `select_column_workaround!` has been removed. If you
  were relying on this internal macro, you can simply delete the line that was
  calling it.

* Columns from the right side of a left join will now need to have `.nullable()`
  explicitly called to be passed to `.select`. This allows it to compose better
  with functions that don't normally take nullable columns (e.g.
  `lower(name).nullable()`).

## [0.11.4] - 2017-02-21

### Fixed

* Corrected a memory safety violation when using MySQL.

## 0.11.3 - 2017-02-21

* No changes

## [0.11.2] - 2017-02-19

### Changed

* `pq-sys` and `mysqlclient-sys` will no longer attempt to generate bindings at
  compile time. Generating the bindings required a bleeding edge version of
  clang, which caused too many issues.

## [0.11.1] - 2017-02-17

### Fixed

* `.on_conflict_do_nothing()` now interacts with slices properly.

* `MysqlConnection` now implements `Send`, which is required for connection
  pooling.

## [0.11.0] - 2017-02-16

### Added

* Added support for MySQL as an additional backend. Diesel CLI will install with
  MySQL support by default. To enable it for Diesel and Diesel Codegen, add
  `features = ["mysql"]` to Cargo.toml. See [the docs][mysql-0.11.0] for details.

[mysql-0.11.0]: http://docs.diesel.rs/diesel/mysql/index.html

* Added support for PG's `ON CONFLICT DO NOTHING` clause. See [the
  docs][on-conflict-0.11.0] for details.

[on-conflict-0.11.0]: http://docs.diesel.rs/diesel/pg/upsert/trait.OnConflictExtension.html#method.on_conflict_do_nothing

* Queries constructed using [`diesel::select`][select-0.11.0] now work properly
  when [boxed][boxed-0.11.0].

[select-0.11.0]: https://docs.rs/diesel/0.11.0/diesel/fn.select.html
[boxed-0.11.0]: http://docs.rs/diesel/0.11.0/prelude/trait.BoxedDsl.html

* Arrays containing null are now supported. `infer_schema!` will never infer an
  array that contains null, but a `table!` definition which specifies a type of
  `Array<Nullable<X>>` can now be deserialized to `Vec<Option<T>>`

* [`#[belongs_to]`][belongs-to-0.11.0] associations can now be self referential.
  This will generate the code required for
  [`belonging_to`][belonging-to-0.11.0], without generating code for performing
  a join.

[belongs-to-0.11.0]: https://docs.rs/diesel/0.11.0/diesel/associations/trait.BelongsTo.html
[belonging-to-0.11.0]: https://docs.rs/diesel/0.11.0/diesel/prelude/trait.BelongingToDsl.html#tymethod.belonging_to

* Added support for the `rust-lang-deprecated/time` crate on PostgreSQL. To use
  it, add `features = ["deprecated-time"]`

### Changed

* It is no longer possible to exhaustively match against
  `result::ConnectionError`.

* Updated chrono to version 0.3.

* [`max`][max-0.11.0] and [`min`][min-0.11.0] are now always nullable. The database will
  return `NULL` when the table is empty.

[max-0.11.0]: http://docs.diesel.rs/diesel/expression/dsl/fn.max.html
[min-0.11.0]: http://docs.diesel.rs/diesel/expression/dsl/fn.min.html

* [`now`][now-0.11.0] can now be used as an expression of type `Timestamptz`.

[now-0.11.0]: http://docs.diesel.rs/diesel/expression/dsl/struct.now.html

* [`Connection::transaction`][transaction-0.11.0] now returns your error
  directly instead of wrapping it in `TransactionError`. It requires that the
  error implement `From<diesel::result::Error>`

[transaction-0.11.0]: http://docs.diesel.rs/diesel/connection/trait.Connection.html#method.transaction

* The way tuples of columns from the right side of left outer joins interact
  with `.select` has changed. If you are deserializing into an option of a tuple
  (instead of a tuple of options), you will need to explicitly call
  `.nullable()`. (e.g. `.select(users::name, (posts::title,
  posts::body).nullable())`)

### Removed

* `result::TransactionError`
* `result::TransactionResult`

## [0.10.1] - 2017-02-08

### Fixed

* `infer_table_from_schema!` properly handles table names with a custom schema
  specified.

### Changed

* Updated uuid to version 0.4.

## [0.10.0] - 2017-02-02

### Added

* Added support for the PostgreSQL [`json` and `jsonb` types][pg-json]. They can
  be mapped to/from `serde_json::Value`. The `serde` feature must be enabled to
  use the JSON types.

[pg-json]: https://www.postgresql.org/docs/9.6/static/datatype-json.html

* Added the `print-schema` command to Diesel CLI. This command will print the
  output of the `infer_schema!` macro. For more information run `diesel help
  print-schema`.

### Changed

* When possible, we will use deprecation warnings for breaking changes.
  Deprecated code requires the `with-deprecated` feature, which is enabled by
  default.

* The `postgres` feature is no longer enabled by default by `diesel` or
  `diesel_codegen_syntex`. Add `features = ["postgres"]` to your `Cargo.toml`.

* The `persistable` module has been renamed to `insertable`.

### Fixed

* `#[derive(Insertable)]` allows fields of type `Option<T>` to be used with
  columns that are not null if they have a default value.

### Removed

* `diesel_codegen_syntex` is no longer supported. `diesel_codegen` can now be
  used on stable Rust.

* Dropped support for Rust 1.14 and earlier

## [0.9.1] - 2016-12-09

### Fixed

* Added missing impls for loading `chrono::NaiveDateTime` from a column of type
  `Timestamptz`

* `#[derive(AsChangeset)]` no longer assumes that `use diesel::prelude::*` has
  been done.

* `debug_sql!` can now properly be used with types from `chrono` or
  `std::time`.

* When using PostgreSQL, attempting to get the error message of a query which
  could not be transmitted to the server (such as a query with greater than
  65535 bind parameters) will no longer panic.

## [0.9.0] - 2016-12-08

### Added

* Added support for SQL `NOT IN` using the `ne_any` method.

* The `table!` macro now allows custom schemas to be specified. Example:

  ```rust
  table! {
    schema_1.table_1 {
      id -> Integer,
    }
  }
  ```

  The generated module will still be called `table_1`.

* The `infer_table_from_schema!` macro now allows custom schemas to be
  specified. Example:

  ```rust
  infer_table_from_schema!("dotenv:DATABASE_URL", "schema_1.table_1");
  ```

* The `infer_schema!` optionally allows a schema name as the second argument. Any
  schemas other than `public` will be wrapped in a module with the same name as
  the schema. For example, `schema_1.table_1` would be referenced as
  `schema_1::table_1`.

* Added support for batch insert on SQLite. This means that you can now pass a
  slice or vector to [`diesel::insert`][insert] on all backends.

[insert]: http://docs.diesel.rs/diesel/fn.insert.html

* Added a function for SQL `EXISTS` expressions. See
  [`diesel::expression::dsl::exists`][exists] for details.

[exists]: http://docs.diesel.rs/diesel/expression/dsl/fn.sql.html

* `#[derive(Identifiable)]` can be used with structs that have primary keys
  other than `id`, as well as structs with composite primary keys. You can now
  annotate the struct with `#[primary_key(nonstandard)]` or `#[primary_key(foo,
  bar)]`.

### Changed

* All macros with the same name as traits we can derive (e.g. `Queryable!`) have
  been renamed to `impl_Queryable!` or similar.

### Fixed

* `#[derive(Identifiable)]` now works on structs with lifetimes

* Attempting to insert an empty slice will no longer panic. It does not execute
  any queries, but the result will indicate that we successfully inserted 0
  rows.

* Attempting to update a record with no changes will no longer generate invalid
  SQL. The result of attempting to execute the query will still be an error, but
  but it will be a `Error::QueryBuilderError`, rather than a database error.
  This means that it will not abort the current transaction, and can be handled
  by applications.

* Calling `eq_any` or `ne_any` with an empty array no longer panics.
  `eq_any(vec![])` will return no rows. `ne_any(vec![])` will return all rows.

## [0.8.2] - 2016-11-22

### Changed

* Fixed support for nightlies later than 2016-11-07

* Removed support for nightlies earlier than 2016-11-07

* Calls to `infer_table_from_schema!` will need to be wrapped in a module if
  called more than once. This change is to work around further limitations of
  the Macros 1.1 system. Example:

  ```rust
  mod infer_users {
      infer_table_from_schema!("dotenv:DATABASE_URL", "users");
  }
  pub use self::infer_users::*;
  ```

## [0.8.1] - 2016-11-01

### Added

* SQLite date and time columns can be deserialized to/from strings.

### Fixed

* Fixed an issue with `diesel_codegen` on nightlies >= 2016-10-20

## [0.8.0] - 2016-10-10

### Added

* Added partial support for composite primary keys.

* Added support for PostgreSQL `NULLS FIRST` and `NULLS LAST` when sorting.
  See http://docs.diesel.rs/diesel/prelude/trait.SortExpressionMethods.html
  for details.

* Added support for the `timestamp with time zone` type in PostgreSQL (referred
  to as `diesel::types::Timestamptz`)

* Diesel CLI can now generate bash completion. See [the readme][bash completion]
  for details.

* `infer_schema!` and `infer_table_from_schema!` can now take `"env:foo"`
  instead of `env!("foo")` and `"dotenv:foo"` instead of `dotenv!("foo")`. The
  use of `dotenv` requires the `dotenv` feature on `diesel_codegen`, which is
  included by default. Using `env!` and `dotenv!` will no longer work with
  `diesel_codegen`. They continue to work with `diesel_codgen_syntex`, but that
  crate will be deprecated when Macros 1.1 is in the beta channel for Rust.

[bash completion]: https://github.com/diesel-rs/diesel/blob/b1a0d9901f0f2a8c8d530ccba8173b57f332b891/diesel_cli/README.md#bash-completion

### Changed

* Structs annotated with `#[has_many]` or `#[belongs_to]` now require
  `#[derive(Associations)]`. This is to allow them to work with Macros 1.1.

* `embed_migrations!` now resolves paths relative to `Cargo.toml` instead of the
  file the macro was called from. This change is required to allow this macro to
  work with Macros 1.1.

### Fixed

* `diesel migrations run` will now respect migration directories overridden by
  command line argument or environment variable
* The `infer_schema!` macro will no longer fetch views alongside with tables.
  This was a source of trouble for people that had created views or are using
  any extension that automatically creates views (e.g. PostGIS)

### Changed

* `#[changeset_for(foo)]` should now be written as
  `#[derive(AsChangeset)] #[table_name="foo"]`. If you were specifying
  `treat_none_as_null = "true"`, you should additionally have
  `#[changeset_options(treat_none_as_null = "true")]`.
* `#[insertable_into(foo)]` should now be written as
  `#[derive(Insertable)] #[table_name="foo"]`.

## [0.7.2] - 2016-08-20

* Updated nightly version and syntex support.

## [0.7.1] - 2016-08-11

### Changed

* The `Copy` constraint has been removed from `Identifiable::Id`, and
  `Identifiable#id` now returns `&Identifiable::Id`.

### Fixed

* `#[belongs_to]` now respects the `foreign_key` option when using
  `diesel_codegen` or `diesel_codegen_syntex`.

## [0.7.0] - 2016-08-01

### Added

* The initial APIs have been added in the form of `#[has_many]` and
  `#[belongs_to]`. See [the module documentation][associations-module] for more
  information.

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

* Added support for `RETURNING` expressions in `DELETE` statements. Implicitly
  these queries will use `RETURNING *`.

### Changed

* Diesel now targets `nightly-2016-07-07`. Future releases will update to a
  newer nightly version on the date that Rust releases.

* `diesel_codegen` has been split into two crates. `diesel_codegen` and
  `diesel_codegen_syntex`. See [this commit][syntex-split] for migration
  information.

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

[associations-module]: http://docs.diesel.rs/diesel/associations/index.html
[syntex-split]: https://github.com/diesel-rs/diesel/commit/36b8801bf5e9594443743e6a7c62e29d3dce36b7

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

## 0.1.0 - 2015-11-29

* Initial release

[0.2.0]: https://github.com/diesel-rs/diesel/compare/v0.1.0...v0.2.0
[0.3.0]: https://github.com/diesel-rs/diesel/compare/v0.2.0...v0.3.0
[0.4.0]: https://github.com/diesel-rs/diesel/compare/v0.3.0...v0.4.0
[0.4.1]: https://github.com/diesel-rs/diesel/compare/v0.4.0...v0.4.1
[0.5.0]: https://github.com/diesel-rs/diesel/compare/v0.4.1...v0.5.0
[0.5.1]: https://github.com/diesel-rs/diesel/compare/v0.5.0...v0.5.1
[0.5.2]: https://github.com/diesel-rs/diesel/compare/v0.5.1...v0.5.2
[0.5.3]: https://github.com/diesel-rs/diesel/compare/v0.5.2...v0.5.3
[0.5.4]: https://github.com/diesel-rs/diesel/compare/v0.5.3...v0.5.4
[0.6.0]: https://github.com/diesel-rs/diesel/compare/v0.5.4...v0.6.0
[0.6.1]: https://github.com/diesel-rs/diesel/compare/v0.6.0...v0.6.1
[0.7.0]: https://github.com/diesel-rs/diesel/compare/v0.6.1...v0.7.0
[0.7.1]: https://github.com/diesel-rs/diesel/compare/v0.7.0...v0.7.1
[0.7.2]: https://github.com/diesel-rs/diesel/compare/v0.7.1...v0.7.2
[0.8.0]: https://github.com/diesel-rs/diesel/compare/v0.7.2...v0.8.0
[0.8.1]: https://github.com/diesel-rs/diesel/compare/v0.8.0...v0.8.1
[0.8.2]: https://github.com/diesel-rs/diesel/compare/v0.8.1...v0.8.2
[0.9.0]: https://github.com/diesel-rs/diesel/compare/v0.8.2...v0.9.0
[0.9.1]: https://github.com/diesel-rs/diesel/compare/v0.9.0...v0.9.1
[0.10.0]: https://github.com/diesel-rs/diesel/compare/v0.9.1...v0.10.0
[0.10.1]: https://github.com/diesel-rs/diesel/compare/v0.10.0...v0.10.1
[0.11.0]: https://github.com/diesel-rs/diesel/compare/v0.10.1...v0.11.0
[0.11.1]: https://github.com/diesel-rs/diesel/compare/v0.11.0...v0.11.1
[0.11.2]: https://github.com/diesel-rs/diesel/compare/v0.11.1...v0.11.2
[0.11.4]: https://github.com/diesel-rs/diesel/compare/v0.11.2...v0.11.4
[0.12.0]: https://github.com/diesel-rs/diesel/compare/v0.11.4...v0.12.0
