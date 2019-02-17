# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## Unreleased

### Added

* `NonAggregate` can now be derived for simple cases.

### Removed

* All previously deprecated items have been removed.

### Changed

* The way [the `Backend` trait][backend-2-0-0] handles its `RawValue` type has
  been changed to allow non-references. Users of this type (e.g. code written
  `&DB::RawValue` or `&<DB as Backend>::RawValue>`) should use
  [`backend::RawValue<DB>`][raw-value-2-0-0] instead. Implementors of `Backend`
  should check the relevant section of [the migration guide][2-0-migration].

[backend-2-0-0]: http://docs.diesel.rs/diesel/backend/trait.Backend.html
[raw-value-2-0-0]: http://docs.diesel.rs/diesel/backend/type.RawValue.html

### Fixed

* Many types were incorrectly considered non-aggregate when they should not
  have been. All types in Diesel are now correctly only considered
  non-aggregate if their parts are.




[2-0-migration]: FIXME write a migration guide

## [1.4.1] - 2019-01-24

### Fixed

* This release fixes a minor memory safety issue in SQLite. This bug would only
  occur in an error handling branch that should never occur in practice.

## [1.4.0] - 2019-01-20

### Fixed

* `embed_migrations!` will no longer emit an unused import warning
* Diesel now supports uuid 0.7 by adding the new feature flag `uuidv07`

### Added

* Diesel CLI can be configured to error if a command would result in changes
  to your schema file by passing `--locked-schema`. This is intended for use
  in CI and production deploys, to ensure that the committed schema file is
  up to date.

* A helper trait has been added for implementing `ToSql` for PG composite types.
  See [`WriteTuple`][write-tuple-1-4-0] for details.

[write-tuple-1-4-0]: docs.diesel.rs/diesel/serialize/trait.WriteTuple.html

* Added support for MySQL's `UNSIGNED TINYINT`

* `DatabaseErrorKind::SerializationFailure` has been added, corresponding to
  SQLSTATE code 40001 (A `SERIALIZABLE` isolation level transaction failed to
  commit due to a read/write dependency on another transaction). This error is
  currently only detected on PostgreSQL.

* Diesel CLI can now generate completions for zsh and fish. See `diesel
  completions --help` for details.

* `#[belongs_to]` can now accept types that are generic over lifetimes (for
  example, if one of the fields has the type `Cow<'a, str>`). To define an
  association to such a type, write `#[belongs_to(parent = "User<'_>")]`

* `Nullable<Text>` now supports `ilike` expression on  in PostgreSQL.

* `diesel_manage_updated_at('table_name')` is now available on SQLite. This
  function can be called in your migrations to create a trigger which
  automatically sets the `updated_at` column, unless that column was updated in
  the query.

### Changed

* Diesel's derives now require that `extern crate diesel;` be at your crate root
  (e.g. `src/lib.rs` or `src/main.rs`)

* `Tinyint` has been renamed to `TinyInt` and an alias has been created from `Tinyint` to `TinyInt`.

* The minimal officially supported rustc version is now 1.31.0

## [1.3.3] - 2018-09-12

### Fixed

* Fixed an issue that occurred with MySQL 8.0 when calling `.execute` or
  `.batch_execute` with a single query that returned a result set (such as our
  `SELECT 1` health check in `r2d2`).

## [1.3.2] - 2018-06-13

### Fixed

* The behavior of unsigned types in MySQL has been corrected to properly set the
  `is_unsigned` flag.

* Fixed an issue with `sql_function!` when `#[sql_name]` was used on functions
  with no return type.

## [1.3.1] - 2018-05-23

### Fixed

* Fixed an issue with Diesel CLI's use of temp files that caused errors on
  Windows.

## [1.3.0] - 2018-05-22

### Added

* Diesel CLI now supports a configuration file. See
  diesel.rs/guides/configuring-diesel-cli for details.

* `sql_function!` now supports generic functions. See [the documentation for
  `sql_function!`][sql-function-1-3-0] for more details.

* `sql_function!` now supports aggregate functions like `sum` and `max`, by
  annotating them with `#[aggregate]`. This skips the implementation of
  `NonAggregate` for your function. See [the documentation for
  `sql_function!`][sql-function-1-3-0] for more details.

* `sql_function!` now supports renaming the function by annotating it with
  `#[sql_name = "SOME_FUNCTION"]`. This can be used to support functions with
  multiple signatures such as coalesce, by defining multiple rust functions
  (with different names) that have the same `#[sql_name]`.

* Added `sqlite-bundled` feature to `diesel_cli` to make installing on
  some platforms easier.

* Custom SQL functions can now be used with SQLite. See [the
  docs][sql-function-sqlite-1-3-0] for details.

[sql-function-sqlite-1-3-0]: http://docs.diesel.rs/diesel/macro.sql_function.html#use-with-sqlite

* All functions and operators provided by Diesel can now be used with numeric
  operators if the SQL type supports it.

* `PgInterval` can now be used with `-`, `*`, and `/`.

* `Vec<T>` is now `Insertable`. It is no longer required to always place an `&`
  in front of `.values`.

* Added support for PG tuples. See [`sql_types::Record`][record-1-3-0] for details.

[record-1-3-0]: http://docs.diesel.rs/diesel/pg/types/sql_types/struct.Record.html

* Added support for a wider range of locking clauses, including `FOR SHARE`,
  `SKIP LOCKED`, `NO WAIT`, and more. See [`QueryDsl`][locking-clause-1-3-0] for details.

[locking-clause-1-3-0]: http://docs.diesel.rs/diesel/query_dsl/trait.QueryDsl.html#method.for_update

### Changed

* `sql_function!` has been redesigned. The syntax is now `sql_function!(fn
  lower(x: Text) -> Text);`. The output of the new syntax is slightly different
  than what was generated in the past. See [the documentation for
  `sql_function!`][sql-function-1-3-0] for more details.

[sql-function-1-3-0]: http://docs.diesel.rs/diesel/macro.sql_function.html

* Diesel's minimum supported Rust version is 1.24.0. This was already true, but
  it is now tested and enforced. Any future changes to our minimum supported
  version will be listed in this change log.

### Fixed

* `diesel print-schema` and `infer_schema!` now properly handle unsigned types
  in MySQL

### Deprecated

* `diesel_infer_schema` has been deprecated. `diesel print-schema` is now the
  only way to generate database schema. Diesel CLI can be configured to
  automatically regenerate your schema file when migrations are run. See
  diesel.rs/guides/configuring-diesel-cli for details.

* Uses of `sql_function!` in the form `sql_function!(foo, foo_t, (x: Integer))`
  have been deprecated in favor of a new design (listed above). Note: Due to [a
  bug in Rust](https://github.com/rust-lang/rust/issues/49912), you may not see
  a deprecation warning from usage of the old form. As always, if you're
  concerned about relying on deprecated code, we recommend attempting to build
  your app with `default-features` turned off (specifically excluding the
  `with-deprecated` feature).

* The `--whitelist` and `--blacklist` options to `diesel print-schema` have been
  deprecated and renamed `--only-tables` and `--exclude-tables`.

## [1.2.2] - 2018-04-12

### Changed

* Warnings are now allowed inside the crate. The way we had attempted to
  deprecate old feature names caused builds to break. We are still not happy
  with how this deprecation gets communicated, and will revisit it in the
  future.

## [1.2.1] - 2018-04-11

### Changed

* Renamed `x32-column-tables`, `x64-column-tables`, and `x128-column-tables` to
  `32-column-tables`, `64-column-tables`, and `128-column-tables`. The leading
  `x` was due to a bug in crates.io discovered while publishing 1.2.0. The bug
  has since been fixed.

## [1.2.0] - 2018-04-06

### Added

* Added `SqlLiteral::bind()`.
  This is intended to be used for binding values to small SQL fragments.
  Use `sql_query` if you are writing full queries.

* Added support for `INSERT INTO table (...) SELECT ...` queries. Tables, select
  select statements, and boxed select statements can now be used just like any
  other `Insertable` value.

* Any insert query written as `insert_into(table).values(values)` can now be
  written as `values.insert_into(table)`. This is particularly useful when
  inserting from a select statement, as select statements tend to span multiple
  lines.

* Diesel's derives can now produce improved error messages if you are using a
  nightly compiler, and enable the `unstable` feature. For the best errors, you
  should also set `RUSTFLAGS="--cfg procmacro2_semver_exempt"`.

* Added support for specifying `ISOLATION LEVEL`, `DEFERRABLE`, and `READ ONLY`
  on PG transactions. See [`PgConnection::build_transaction`] for details.

[`PgConnection::build_transaction`]: http://docs.diesel.rs/diesel/pg/struct.PgConnection.html#method.build_transaction

* Added support for `BEGIN IMMEDIATE` and `BEGIN EXCLUSIVE` on SQLite.
  See [`SqliteConnection::immediate_transaction`] and
  [`SqliteConnection::exclusive_transaction`] for details

[`SqliteConnection::immediate_transaction`]: http://docs.diesel.rs/diesel/sqlite/struct.SqliteConnection.html#method.immediate_transaction
[`SqliteConnection::exclusive_transaction`]: http://docs.diesel.rs/diesel/sqlite/struct.SqliteConnection.html#method.exclusive_transaction

* Tables with more than 56 columns are now supported by enabling the
  `128-column-tables` feature.

* Delete statements can now be boxed. This is useful for conditionally modifying
  the where clause of a delete statement. See [`DeleteStatement::into_boxed`]
  for details.

[`DeleteStatement::into_boxed`]: http://docs.diesel.rs/diesel/query_builder/struct.DeleteStatement.html#method.into_boxed

* Update statements can now be boxed. This is useful for conditionally modifying
  the where clause of a update statement. See [`UpdateStatement::into_boxed`]
  for details.

[`UpdateStatement::into_boxed`]: http://docs.diesel.rs/diesel/query_builder/struct.UpdateStatement.html#method.into_boxed

* Added `order_by` as an alias for `order`.

* Added `then_order_by`, which appends to an `ORDER BY` clause rather than
  replacing it. This is useful with boxed queries to dynamically construct an
  order by clause containing an unknown number of columns.

* `#[derive(Insertable)]` can now work on structs with fields that implement
  `Insertable` (meaning one field can map to more than one column). Add
  `#[diesel(embed)]` to the field to enable this behavior.

* Queries that treat a subselect as a single value (e.g. `foo = (subselect)`)
  are now supported by calling [`.single_value()`].

* `#[derive(Insertable)]` implements now `Insertable` also on the struct itself,
  not only on references to the struct

[`.single_value()`]: http://docs.diesel.rs/diesel/query_dsl/trait.QueryDsl.html#method.single_value

* `ConnectionError` now implements `PartialEq`.

* Columns generated by `table!` now implement `Default`

* `#[derive(AsChangeset)]` now implements `AsChangeset` on the struct itself,
  and not only on a reference to the struct

* Added support for deserializing `Numeric` into `BigDecimal` on SQLite. SQLite
  has no arbitrary precision type, so the result will still have floating point
  rounding issues. This is primarily to support things like `avg(int_col)`,
  which we define as returning `Numeric`

### Changed

* The bounds on `impl ToSql for Cow<'a, T>` have been loosened to no longer
  require that `T::Owned: ToSql`.

* `32-column-tables` are now enabled by default.

### Deprecated

* `ne_any` has been renamed to `ne_all`.

* The `large-tables` feature has been has been renamed to `32-column-tables`.

* The `huge-tables` feature has been renamed to `64-column-tables`.

* `IncompleteUpdateStatement` has been removed. Use `UpdateStatement` instead.

### Fixed

* `diesel database setup` now correctly handles database URLs containing query
  strings

* `diesel migration list` shows the proper migration order when mixing
  old and new timestamp formats. (The migrations were always run in the correct
  order, this only affects the display logic of `migration list`)

* `#[derive(Identifiable)]` now correctly associates `#[primary_key]` with the
  column name, not field name.

* Select statements can no longer incorrectly appear in an expression context.

* `exists` can no longer incorrectly receive values other than select
  statements.

* `MysqlConnection::establish` can now properly handle IPv6 addresses wrapped in
  square brackets.

### Jokes

* Diesel is now powered by the blockchain because it's 2018.

## [1.1.2] - 2018-04-05

* No changes

## [1.1.1] - 2018-01-16

### Added

* Added `diesel::r2d2::PoolError` as an alias for `r2d2::Error`. Previously this
  type was inaccessible due to `diesel::r2d2::Error`.

## [1.1.0] - 2018-01-15

### Added

* `r2d2-diesel` has been merged into Diesel proper. You should no longer rely
  directly on `r2d2-diesel` or `r2d2`. The functionality of both is exposed from
  `diesel::r2d2`.

* `r2d2::PooledConnection` now implements `Connection`. This means that you
  should no longer need to write `&*connection` when using `r2d2`.

* The `BINARY` column type name is now supported for SQLite.

* The `QueryId` trait can now be derived.

* `FromSqlRow` can now be derived for types which implement `FromSql`.

* `AsExpression` can now be derived for types which implement `ToSql`.

* `HasSqlType`, `NotNull`, and `SingleValue` can now be derived with
  `#[derive(SqlType)]`. See the docs for those traits for more information.

* The return type of `FromSql`, `FromSqlRow`, and `QueryableByName` can now be
  written as `deserialize::Result<Self>`.

* The return type of `ToSql` can now be written as `serialize::Result`.

* Added support for SQLite's `INSERT OR IGNORE` and MySQL's `INSERT IGNORE`
  via the `insert_or_ignore` function.

* `min` and `max` can now be used with array expressions.

* Added `diesel::dsl::array`, which corresponds to a PG `ARRAY[]` literal.

* Added the `not_none!` macro, used by implementations of `FromSql` which do not
  expect `NULL`.

* Added `result::UnexpectedNullError`, an `Error` type indicating that an
  unexpected `NULL` was received during deserialization.

* Added `.or_filter`, which behaves identically to `.filter`, but using `OR`
  instead of `AND`.

* `helper_types` now contains a type for every method defined in
  `expression_methods`, and every function in `dsl`.

* Added `FromSql` impls for `*const str` and `*const [u8]` everywhere that
  `String` and `Vec` are supported. These impls do not allocate, and are
  intended for use by other impls which need to parse a string or bytes, and
  don't want to allocate. These impls should never be used outside of another
  `FromSql` impl.

### Deprecated

* *IMPORTANT NOTE* Due to [several][rust-deprecation-bug-1]
  [bugs][rust-deprecation-bug-2] in Rust, many of the deprecations in this
  release may not show a warning. If you want to ensure you are not using any
  deprecated items, we recommend attempting to compile your code without the
  `with-deprecated` feature by adding `default-features = false` to
  `Cargo.toml`.

[rust-deprecation-bug-1]: https://github.com/rust-lang/rust/issues/47236
[rust-deprecation-bug-2]: https://github.com/rust-lang/rust/issues/47237

* Deprecated `impl_query_id!` in favor of `#[derive(QueryId)]`

* Deprecated specifying a column name as `#[column_name(foo)]`. `#[column_name =
  "foo"]` should be used instead.

* The `types` module has been deprecated. It has been split into `sql_types`,
  `serialize`, and `deserialize`.

* `query_source::Queryable` and `query_source::QueryableByName` have been
  deprecated. These traits have been moved to `deserialize`.

* `backend::TypeMetadata` has been deprecated. It has been moved to `sql_types`.

* `types::ToSqlOutput` has been deprecated. It has been renamed to
  `serialize::Output`.

* `helper_types::Not` is now `helper_types::not`

### Fixed

* `infer_schema!` generates valid code when run against a database with no
  tables.

## [1.0.0] - 2018-01-02

### Added

* `#[derive(QueryableByName)]` can now handle structs that have no associated
  table. If the `#[table_name]` annotation is left off, you must annotate each
  field with `#[sql_type = "Integer"]`

* `#[derive(QueryableByName)]` can now handle embedding other structs. To have a
  field whose type is a struct which implements `QueryableByName`, rather than a
  single column in the query, add the annotation `#[diesel(embed)]`

* The `QueryDsl` trait encompasses the majority of the traits that were
  previously in the `query_dsl` module.

### Fixed

* Executing select statements on SQLite will no longer panic when the database
  returns `SQLITE_BUSY`

* `table!`s which use the `Datetime` type with MySQL will now compile correctly,
  even without the `chrono` feature enabled.

* `#[derive(QueryableByName)]` will now compile correctly when there is a shadowed `Result` type in scope.

* `BoxableExpression` can now be used with types that are not `'static`

### Changed

* `Connection::test_transaction` now requires that the error returned implement `Debug`.

* `query_builder::insert_statement::InsertStatement` is now accessed as
  `query_builder::InsertStatement`

* `query_builder::insert_statement::UndecoratedInsertRecord` is now accessed as
  `query_builder::UndecoratedInsertRecord`

* `#[derive(QueryableByName)]` now requires that the table name be explicitly
  stated.

* Most of the traits in `query_dsl` have been moved to `query_dsl::methods`.
  These traits are no longer exported in `prelude`. This should not affect most
  apps, as the behavior of these traits is provided by `QueryDsl`. However, if
  you were using these traits in `where` clauses for generic code, you will need
  to explicitly do `use diesel::query_dsl::methods::WhateverDsl`. You may also
  need to use UFCS in these cases.

* If you have a type which implemented `QueryFragment` or `Query`, which you
  intended to be able to call `execute` or `load` on, you will need to manually
  implement `RunQueryDsl` for that type. The trait should be unconditionally
  implemented (no where clause beyond what your type requires), and the body
  should be empty.

### Removed

* All deprecated items have been removed.

* `LoadDsl` and `FirstDsl` have been removed. Their functionality now lives in
  `LoadQuery`.

## [0.99.1] - 2017-12-01

### Changed

* Diesel CLI now properly restricts its `clap` dependency. 0.99.0 mistakenly had
  no upper bound on the version.

## [0.99.0] - 2017-11-28

### Added

* The `.for_update()` method has been added to select statements, allowing
  construction of `SELECT ... FOR UPDATE`.

* Added `insert_into(table).default_values()` as a replacement for
  `insert_default_values()`

* Added `insert_into(table).values(values)` as a replacement for
  `insert(values).into(table)`.

* Added support for MySQL's `REPLACE INTO` as `replace_into(table)`.

* Added `replace_into(table).values(values)` as a replacement for
  `insert_or_replace(values).into(table)`.

* Added `on_conflict_do_nothing` on `InsertStatement` as a replacement for
  `on_conflict_do_nothing` on `Insertable` structs.

* Added `on_conflict` on `InsertStatement` as a replacement for
  `on_conflict` on `Insertable` structs.

* `filter` can now be called on update and delete statements. This means that
  instead of `update(users.filter(...))` you can write
  `update(users).filter(...)`. This allows line breaks to more naturally be
  introduced.

* Subselects can now reference columns from the outer table. For example,
  `users.filter(exists(posts.filter(user_id.eq(users::id))))` will now compile.

* `TextExpressionMethods` is now implemented for expressions of type
  `Nullable<Text>` as well as `Text`.

* `allow_tables_to_appear_in_same_query!` can now take more than 2 tables, and is the same
  as invoking it separately for every combination of those tables.

* Added `sql_query`, a new API for dropping to raw SQL that is more pleasant to
  use than `sql` for complete queries. The main difference from `sql` is that
  you do not need to state the return type, and data is loaded from the query by
  name rather than by index.

* Added a way to rename a table in the `table!` macro with `#[sql_name="the_table_name"]`

* Added support for PostgreSQL's `DISTINCT ON`. See
  [`.distinct_on()`][0.99.0-distinct-on] for more details

### Changed

* The signatures of `QueryId`, `Column`, and `FromSqlRow` have all changed to
  use associated constants where appropriate.

* You will now need to invoke `allow_tables_to_appear_in_same_query!` any time two tables
  appear together in the same query, even if there is a `joinable!` invocation for those tables.

* `diesel_codegen` should no longer explicitly be used as a dependency. Unless
  you are using `infer_schema!` or `embed_migrations!`, you can simply remove it
  from your `Cargo.toml`. All other functionality is now provided by `diesel`
  itself.

* Code using `infer_schema!` or `infer_table_from_schema!` must now add
  `diesel_infer_schema` to `Cargo.toml`, and `#[macro_use] extern crate
  diesel_infer_schema` to `src/lib.rs`

* Code using `embed_migrations!` must now add `diesel_migrations` to `Cargo.toml`,
  and `#[macro_use] extern crate diesel_migrations` to `src/lib.rs`

* The `migrations` module has been moved out of `diesel` and into
  `diesel_migrations`

### Deprecated

* Deprecated `insert_default_values()` in favor of
  `insert_into(table).default_values()`

* Deprecated `insert(values).into(table)` in favor of
  `insert_into(table).values(values)`.

* Deprecated `insert_or_replace(values).into(table)` in favor of
  `replace_into(table).values(values)`.

* Deprecated `.values(x.on_conflict_do_nothing())` in favor of
  `.values(x).on_conflict_do_nothing()`

* Deprecated `.values(x.on_conflict(y, do_nothing()))` in favor of
  `.values(x).on_conflict(y).do_nothing()`

* Deprecated `.values(x.on_conflict(y, do_update().set(z)))` in favor of
  `.values(x).on_conflict(y).do_update().set(z)`

* Deprecated `enable_multi_table_joins` in favor of
  `allow_tables_to_appear_in_same_query!`

* Deprecated `SqlLiteral#bind`. `sql` is intended for use with small fragments
  of SQL, not complete queries. Writing bind parameters in raw SQL when you are
  not writing the whole query is error-prone. Use `sql_query` if you need raw
  SQL with bind parameters.

### Removed

* `IntoInsertStatement` and `BatchInsertStatement` have been removed. It's
  unlikely that your application is using these types, but `InsertStatement` is
  now the only "insert statement" type.

* `Citext` as a type alias for `Text` has been removed. Writing
  `citext_column.eq("foo")` would perform a case-sensitive comparison. More
  fleshed out support will be required.

### Fixed

* When using MySQL and SQLite, dates which cannot be represented by `chrono`
  (such as `0000-00-00`) will now properly return an error instead of panicking.

* MySQL URLs will now properly percent decode the username and password.

* References to types other than `str` and slice can now appear on structs which
  derive `Insertable` or `AsChangeset`.

* Deserializing a date/time/timestamp column into a chrono type on SQLite will
  now handle any value that is in a format documented as valid for SQLite's
  `strftime` function except for the string `'now'`.

[0.99.0-distinct-on]: http://docs.diesel.rs/diesel/query_dsl/trait.DistinctOnDsl.html#tymethod.distinct_on

## [0.16.0] - 2017-08-24

### Added

* Added helper types for inner join and left outer join

* `diesel::debug_query` has been added as a replacement for `debug_sql!`. This
  function differs from the macro by allowing you to specify the backend, and
  will generate the actual query which will be run. The returned value will
  implement `Display` and `Debug` to show the query in different ways

* `diesel::pg::PgConnection`, `diesel::mysql::MysqlConnection`, and
  `diesel::sqlite::SqliteConnection` are now exported from `diesel::prelude`.
  You should no longer need to import these types explicitly.

* Added support for the Decimal datatype on MySQL, using the [BigDecimal crate][bigdecimal-0.16.0].

* Added support for the [Range][range-0.16.0] type on postgreSQL.

* Added support for the Datetime type on MySQL.

* Added support for the Blob type on MySQL.

* `infer_schema!` will now automatically detect which tables can be joined based
  on the presence of foreign key constraints.

* Added support for `Add` and `Sub` to timestamp types.

* Added a way to rename columns in the table macro with `#[sql_name="the_column_name"]`

* Schema inference now also generates documentation comments for tables and
  columns. For `infer_schema!`, this is enabled by default. If you are using
  Diesel's CLI tool, pass the new `--with-docs` parameter:
  `diesel print-schema --with-docs`.

* `infer_schema!` now automatically renames columns that conflict with
  a Rust keyword by placing a _ at the end of the name. For example,
  a column called `type` will be referenced as `type_` in Rust.

### Changed

* The deprecated `debug_sql!` and `print_sql!` functions will now generate
  backend specific SQL. (The specific backend they will generate for will be
  arbitrarily chosen based on the backends enabled).

* `#[belongs_to]` will no longer generate the code required to join between two
  tables. You will need to explicitly invoke `joinable!` instead, unless you are
  using `infer_schema!`

* Changed the migration directory name format to `%Y-%m-%d-%H%M%S`.

* `between` and `not_between` now take two arguments, rather than a range.

### Removed

* `debug_sql!` has been deprecated in favor of `diesel::debug_query`.

* `print_sql!` has been deprecated without replacement.

* `diesel::backend::Debug` has been removed.

### Fixed

* Diesel now properly supports joins in the form:
  `grandchild.join(child.join(parent))`. Previously only
  `parent.join(child.join(grandchild))` would compile.

* When encoding a `BigDecimal` on PG, `1.0` is no longer encoded as if it were
  `1`.

[bigdecimal-0.16.0]: https://crates.io/crates/bigdecimal
[range-0.16.0]: https://docs.diesel.rs/diesel/pg/types/sql_types/struct.Range.html

## [0.15.2] - 2017-07-28

### Fixed

* `BigDecimal` now properly encodes numbers starting with `10000` on postgres.
  See [issue #1044][] for details.

[issue #1044]: https://github.com/diesel-rs/diesel/issues/1044

## [0.15.1] - 2017-07-24

* No changes to public API

## [0.15.0] - 2017-07-23

### Added

* Added support for the PG `IS DISTINCT FROM` operator

* The `ON` clause of a join can now be manually specified. See [the
  docs][join-on-dsl-0.15.0] for details.

[join-on-dsl-0.15.0]: https://docs.diesel.rs/diesel/prelude/trait.JoinOnDsl.html#method.on

### Changed

* Diesel will now automatically invoke `numeric_expr!` for your columns in the
  common cases. You will likely need to delete any manual invocations of this
  macro.

* `Insertable` no longer treats all fields as nullable for type checking. What
  this means for you is that if you had an impl like `impl
  AsExpression<Nullable<SqlType>, DB> for CustomType` in your code base, you can
  remove the `Nullable` portion (Unless you are using it with fields that are
  actually nullable)

* Connections will now explicitly set the session time zone to UTC when the
  connection is established

## [0.14.1] - 2017-07-10

### Changed

* The return type of `sum` and `avg` is now always considered to be `Nullable`,
  as these functions return `NULL` when against on an empty table.

## [0.14.0] - 2017-07-04

### Added

* Added support for joining between more than two tables. The query builder can
  now be used to join between any number of tables in a single query. See the
  documentation for [`JoinDsl`][join-dsl-0.14.0] for details

[join-dsl-0.14.0]: https://docs.diesel.rs/diesel/prelude/trait.JoinDsl.html

* Added support for the [PostgreSQL network types][pg-network-0.14.0] `MACADDR`.

* Added support for the Numeric datatypes, using the [BigDecimal crate][bigdecimal-0.14.0].

* Added a function which maps to SQL `NOT`. See [the docs][not-0.14.0] for more
  details.

* Added the [`insert_default_values`][insert-default-0.14.0] function.

[pg-network-0.14.0]: https://www.postgresql.org/docs/9.6/static/datatype-net-types.html
[not-0.14.0]: https://docs.diesel.rs/diesel/expression/dsl/fn.not.html
[insert-default-0.14.0]: https://docs.diesel.rs/diesel/fn.insert_default_values.html
[bigdecimal-0.14.0]: https://crates.io/crates/bigdecimal

* Added `diesel_prefix_operator!` which behaves identically to
  `diesel_postfix_operator!` (previously `postfix_predicate!`), but for
  operators like `NOT` which use prefix notation.

### Changed

* `infix_predicate!` and `infix_expression!` have been renamed to
  `diesel_infix_operator!`.

* `postfix_predicate!` and `postfix_expression!` have been renamed to
  `diesel_postfix_operator!`.

* Trait bounds along the lines of `T: LoadDsl<Conn>, U: Queryable<T::SqlType,
  Conn::Backend>` should be changed to `T: LoadQuery<Conn, U>`.

* Diesel now uses a migration to set up its timestamp helpers. To generate this
  migration for your project, run `diesel database setup`.

### Removed

* `#[has_many]` has been removed. Its functionality is now provided by
  `#[belongs_to]` on the child struct. If there is no child struct to
  put `#[belongs_to]` on, you can invoke `joinable!` directly instead.

## [0.13.0] - 2017-05-15

### Added

* Added support for chrono types with SQLite.

* Bind values can now be supplied to queries constructed using raw SQL. See [the
  docs][sql-bind-0.13.0] for more details.

[sql-bind-0.13.0]: https://docs.diesel.rs/diesel/expression/sql_literal/struct.SqlLiteral.html#method.bind

* Added support for the [PostgreSQL network types][pg-network-0.13.0] `CIDR` and
  `INET`.

[pg-network-0.13.0]: https://www.postgresql.org/docs/9.6/static/datatype-net-types.html

* Added support for `ILIKE` in PostgreSQL.

* `diesel migration list` will show all migrations, marking those that have been
  run.

* `diesel migration pending` will list any migrations which have not been run.

* Added support for numeric operations with nullable types.

* Added [`migrations::any_pending_migrations`][pending-migrations-0.13.0].

[pending-migrations-0.13.0]: https://docs.diesel.rs/diesel/migrations/fn.any_pending_migrations.html

### Fixed

* Diesel CLI now respects the `--migration-dir` argument or the
  `MIGRATION_DIRECTORY` environment variable for all commands.

* Diesel CLI now properly escapes the database name.

## [0.12.1] - 2017-05-07

### Changed

* Locked the chrono dependency to require exactly `0.3.0` instead of a semver
  restriction. This restriction is required for the 0.12 line of releases to
  continue compiling, as the chrono project is including breaking changes in
  patch releases.

## [0.12.0] - 2017-03-16

### Added

* Added support for the majority of PG upsert (`INSERT ON CONFLICT`). We now
  support specifying the constraint, as well as `DO UPDATE` in addition to `DO
  NOTHING`. See [the module docs][upsert-0.12.0] for details.

[upsert-0.12.0]: https://docs.diesel.rs/diesel/pg/upsert/index.html

* Added support for the SQL concatenation operator `||`. See [the docs for
  `.concat`][concat-0.12.0] for more details.

[concat-0.12.0]: https://docs.diesel.rs/diesel/expression/expression_methods/text_expression_methods/trait.TextExpressionMethods.html#method.concat

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

[mysql-0.11.0]: https://docs.diesel.rs/diesel/mysql/index.html

* Added support for PG's `ON CONFLICT DO NOTHING` clause. See [the
  docs][on-conflict-0.11.0] for details.

[on-conflict-0.11.0]: https://docs.diesel.rs/diesel/pg/upsert/trait.OnConflictExtension.html#method.on_conflict_do_nothing

* Queries constructed using [`diesel::select`][select-0.11.0] now work properly
  when [boxed][boxed-0.11.0].

[select-0.11.0]: https://docs.rs/diesel/0.11.0/diesel/fn.select.html
[boxed-0.11.0]: https://docs.rs/diesel/0.11.0/prelude/trait.BoxedDsl.html

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

[max-0.11.0]: https://docs.diesel.rs/diesel/expression/dsl/fn.max.html
[min-0.11.0]: https://docs.diesel.rs/diesel/expression/dsl/fn.min.html

* [`now`][now-0.11.0] can now be used as an expression of type `Timestamptz`.

[now-0.11.0]: https://docs.diesel.rs/diesel/expression/dsl/struct.now.html

* [`Connection::transaction`][transaction-0.11.0] now returns your error
  directly instead of wrapping it in `TransactionError`. It requires that the
  error implement `From<diesel::result::Error>`

[transaction-0.11.0]: https://docs.diesel.rs/diesel/connection/trait.Connection.html#method.transaction

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

[insert]: https://docs.diesel.rs/diesel/fn.insert.html

* Added a function for SQL `EXISTS` expressions. See
  [`diesel::expression::dsl::exists`][exists] for details.

[exists]: https://docs.diesel.rs/diesel/expression/dsl/fn.sql.html

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
  See https://docs.diesel.rs/diesel/prelude/trait.SortExpressionMethods.html
  for details.

* Added support for the `timestamp with time zone` type in PostgreSQL (referred
  to as `diesel::types::Timestamptz`)

* Diesel CLI can now generate bash completion. See [the readme][bash completion]
  for details.

* `infer_schema!` and `infer_table_from_schema!` can now take `"env:foo"`
  instead of `env!("foo")` and `"dotenv:foo"` instead of `dotenv!("foo")`. The
  use of `dotenv` requires the `dotenv` feature on `diesel_codegen`, which is
  included by default. Using `env!` and `dotenv!` will no longer work with
  `diesel_codegen`. They continue to work with `diesel_codegen_syntex`, but that
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
  https://docs.diesel.rs/diesel/macro.Insertable!.html for details.

* The `Queryable!` macro can now be used instead of `#[derive(Queryable)]` for
  those wishing to avoid syntax extensions from `diesel_codegen`. See
  https://docs.diesel.rs/diesel/macro.Queryable!.html for details.

* The `Identifiable!` macro can now be used instead of `#[derive(Identifiable)]` for
  those wishing to avoid syntax extensions from `diesel_codegen`. See
  https://docs.diesel.rs/diesel/macro.Identifiable!.html for details.

* The `AsChangeset!` macro can now be used instead of `#[changeset_for(table)]`
  for those wishing to avoid syntax extensions from `diesel_codegen`. See
  https://docs.diesel.rs/diesel/macro.AsChangeset!.html for details.

* Added support for the PostgreSQL `ALL` operator. See
  https://docs.diesel.rs/diesel/pg/expression/dsl/fn.all.html for details.

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
  https://docs.diesel.rs/diesel/result/enum.Error.html and
  https://docs.diesel.rs/diesel/result/trait.DatabaseErrorInformation.html for
  more information

* Structs which implement `Identifiable` can now be passed to `update` and
  `delete`. This means you can now write `delete(&user).execute(&connection)`
  instead of `delete(users.find(user.id)).execute(&connection)`

[associations-module]: https://docs.diesel.rs/diesel/associations/index.html
[syntex-split]: https://github.com/diesel-rs/diesel/commit/36b8801bf5e9594443743e6a7c62e29d3dce36b7

### Fixed

* `&&[T]` can now be used in queries. This allows using slices with things like
  `#[insertable_into]`.

## [0.6.1] 2016-04-14

### Added

* Added the `escape` method to `Like` and `NotLike`, to specify the escape
  character used in the pattern. See [EscapeExpressionMethods][escape] for
  details.

[escape]: https://docs.diesel.rs/diesel/expression/expression_methods/escape_expression_methods/trait.EscapeExpressionMethods.html

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

[boxed_dsl]: https://docs.diesel.rs/diesel/prelude/trait.BoxedDsl.html

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
  the [migrations](https://docs.diesel.rs/diesel/migrations/index.html)
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
  more idiomatic than checking for `Err(NotFound)`.

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
[0.12.1]: https://github.com/diesel-rs/diesel/compare/v0.12.0...v0.12.1
[0.13.0]: https://github.com/diesel-rs/diesel/compare/v0.12.1...v0.13.0
[0.14.0]: https://github.com/diesel-rs/diesel/compare/v0.13.0...v0.14.0
[0.14.1]: https://github.com/diesel-rs/diesel/compare/v0.14.0...v0.14.1
[0.15.0]: https://github.com/diesel-rs/diesel/compare/v0.14.1...v0.15.0
[0.15.1]: https://github.com/diesel-rs/diesel/compare/v0.15.0...v0.15.1
[0.15.2]: https://github.com/diesel-rs/diesel/compare/v0.15.1...v0.15.2
[0.16.0]: https://github.com/diesel-rs/diesel/compare/v0.15.2...v0.16.0
[0.99.0]: https://github.com/diesel-rs/diesel/compare/v0.16.0...v0.99.0
[0.99.1]: https://github.com/diesel-rs/diesel/compare/v0.99.0...v0.99.1
[1.0.0]: https://github.com/diesel-rs/diesel/compare/v0.99.1...v1.0.0
[1.1.0]: https://github.com/diesel-rs/diesel/compare/v1.0.0...v1.1.0
[1.1.1]: https://github.com/diesel-rs/diesel/compare/v1.1.0...v1.1.1
[1.1.2]: https://github.com/diesel-rs/diesel/compare/v1.1.1...v1.1.2
[1.2.0]: https://github.com/diesel-rs/diesel/compare/v1.1.2...v1.2.0
[1.2.1]: https://github.com/diesel-rs/diesel/compare/v1.2.0...v1.2.1
[1.2.2]: https://github.com/diesel-rs/diesel/compare/v1.2.1...v1.2.2
[1.3.0]: https://github.com/diesel-rs/diesel/compare/v1.2.2...v1.3.0
[1.3.1]: https://github.com/diesel-rs/diesel/compare/v1.3.0...v1.3.1
[1.3.2]: https://github.com/diesel-rs/diesel/compare/v1.3.1...v1.3.2
[1.3.3]: https://github.com/diesel-rs/diesel/compare/v1.3.2...v1.3.3
[1.4.0]: https://github.com/diesel-rs/diesel/compare/v1.3.0...v1.4.0
[1.4.1]: https://github.com/diesel-rs/diesel/compare/v1.4.0...v1.4.1
