Diesel Codegen
============

Provides various macros and annotations for
[Diesel](http://sgrif.github.io/diesel/diesel/index.html) to reduce the amount of
boilerplate needing to be written. It can be used through `rustc_plugin`, or
`syntex` on stable.

Using on nightly
----------------

Make sure you're on a nightly from 2015-11-27 or later, we don't compile on earlier versions. To use with nightly, you'll want to turn off the default features. Add this
line to your dependencies section in `Cargo.toml`

```toml
diesel_codegen = { version = "^0.4.0", default-features = false, features = ["nightly"] }
```

Then you'll need to add two lines to the root of your crate.

```rust
#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen)]
```

After that, you'll be good to go.

Using on stable
---------------

On stable, you'll need to use [`syntex`](https://crates.io/crates/syntex) to
build any modules using our annotations. Add the following to your
build-dependencies.

```toml
diesel_codegen = "^0.4.0"
syntex = "^0.26.0"
```

You'll need to move any code using annotations into a different file.

`src/schema.rs`

```rust
include!(concat!(env!("OUT_DIR"), "/schema.rs"));
```

`src/schema.in.rs`

```rust
#[derive(Queryable)]
pub struct User {
    id -> i32,
    name -> String,
}
```

Then create a `build.rs` with the following:

```rust
extern crate syntex;
extern crate diesel_codegen;

use std::env;
use std::path::Path;

pub fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut registry = syntex::Registry::new();
    diesel_codegen::register(&mut registry);

    let src = Path::new("src/schema.in.rs");
    let dst = Path::new(&out_dir).join("schema.rs");

    registry.expand("", &src, &dst).unwrap();
}
```

Note that compiler errors will be reported in the generated file, not the source
file. For that reason, it's often easier to develop with nightly rust, and
deploy or test on stable. You can see an example of how to do this by looking at
[Diesel's tests](https://github.com/sgrif/diesel/tree/master/diesel_tests).

Struct annotations
------------------

### `#[derive(Queryable)]`

Adds an implementation of the [`Queryable`][queryable] trait to the annotated
item. At this time it only supports structs with named fields. Enums and tuple
structs are not supported.

### `#[insertable_into(table_name)]`

Adds an implementation of the [`Insertable`][insertable] trait to the annotated
item, targeting the given table. Can only annotate structs and tuple structs.
Enums are not supported. See [field annotations][#field-annotations] for
additional configurations.

### `#[changeset_for(table_name)]`

Adds an implementation of the [`AsChangeset`][as_changeset] trait to the
annotated item, targeting the given table. At this time, it only supports
structs with named fields. Tuple structs and enums are not supported. See [field
annotations][#field-annotations] for additional configurations.

Any fields which are of the type `Option` will be skipped when their value is
`None`. This makes it easy to support APIs where you may not want to update all
of the fields of a record on every request.

If you'd like `None` to change a field to `NULL`, instead of skipping it, you
can pass the `treat_none_as_null` option like so: `#[changeset_for(posts,
treat_none_as_null="true")]`

If the struct has a field for the primary key, an additional function,
`save_changes<T: Queryable<..>>(&self, connection: &Connection) ->
QueryResult<T>`, will be added to the model. This will persist any changes made,
and return the resulting record. It is intended to be a shorthand for filtering
by the primary key.

[queryable]: http://sgrif.github.io/diesel/diesel/query_source/trait.Queryable.html
[insertable]: http://sgrif.github.io/diesel/diesel/trait.Insertable.html
[as_changeset]: http://sgrif.github.io/diesel/diesel/query_builder/trait.AsChangeset.html

Field annotations
-----------------

### `#[column_name="value"]`

Any field can be annotated with `column_name=` to have it map to a column with a
different name. This is required for all fields of tuple structs.

Macros (Experimental)
---------------------

### `infer_schema!("database_url")`

Queries the database for the names of all tables, and calls
`infer_table_from_schema!` for each one. We recommend using with the
[`dotenv`](https://github.com/slapresta/rust-dotenv) crate, and invoking this as
`infer_schema!(dotenv!("DATABASE_URL"))`

### `infer_table_from_schema!("database_url", "table_name")`

Establishes a database connection at compile time, loads the schema information
about a table's columns, and invokes
[`table`](http://sgrif.github.io/diesel/diesel/macro.table!.html) for you
automatically. We recommend using with the
[`dotenv`](https://github.com/slapresta/rust-dotenv) crate, and invoking this as
`infer_table_from_schema!(dotenv!("DATABASE_URL"), "table_name")`

At this time, the schema inference macros do not support types from third party
crates, and having any columns with a type not already supported will result in
a compiler error (please open an issue if this happens unexpectedly for a type
listed in [our
docs](http://sgrif.github.io/diesel/diesel/types/index.html#structs).)
