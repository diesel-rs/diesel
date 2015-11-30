# Change Log
All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## Unreleased

### Fixed

* `#[derive(Queriable)]` now allows generic parameters on the struct.

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

[insert]: http://sgrif.github.io/diesel/diesel/query_builder/fn.insert.html

## [0.1.0] - 2015-11-29

* Initial release
