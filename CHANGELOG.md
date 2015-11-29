# Change Log
All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)


## Unreleased

### Changed

* Added a hidden `__Nonexhaustive` variant to `result::Error`. This is not
  intended to be something you can exhaustively match on, but I do want people
  to be able to check for specific cases, so `Box<std::error::Error>` is
  not an option.

## [0.1.0] - 2015-11-29

* Initial release
