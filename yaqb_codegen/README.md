YAQB Codegen
============

Provides various macros and annotations for
[YAQB](http://sgrif.github.io/yaqb/yaqb/index.html) to reduce the amount of
boilerplate needing to be written. It can be used through `rustc_plugin`, or
`syntex` on stable.

Using on nightly
----------------

To use with nightly, you'll want to turn off the default features. Add this
line to your dependencies section in `Cargo.toml`

```toml
yaqb_codegen = { version = "^0.1.0", default-features = false, features = ["nightly"] }
```

Then you'll need to add two lines to the root of your crate.

```rust
#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(yaqb_codegen)]
```

After that, you'll be good to go.

Using on stable
---------------

On stable, you'll need to use [`syntex`](https://crates.io/crates/syntex) to
build any modules using our annotations. Add the following to your
build-dependencies.

```toml
yaqb_codegen = "^0.1.0"
syntex = "^0.22.0"
```

You'll need to move any code using annotations into a different file.

`src/schema.rs`

```rust
include!(concat!(env!("OUT_DIR"), "/schema.rs"));
```

`src/schema.in.rs`

```rust
#[derive(Queriable)]
pub struct User {
    id -> i32,
    name -> String,
}
```

Then create a `build.rs` with the following:

```rust
extern crate syntex;
extern crate yaqb_codegen;

use std::env;
use std::path::Path;

pub fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut registry = syntex::Registry::new();
    yaqb_codegen::register(&mut registry);

    let src = Path::new("src/schema.in.rs");
    let dst = Path::new(&out_dir).join("schema.rs");

    registry.expand("", &src, &dst).unwrap();
}
```

Note that compiler errors will be reported in the generated file, not the source
file. For that reason, it's often easier to develop with nightly rust, and
deploy or test on stable. You can see an example of how to do this by looking at
[YAQB's tests](https://github.com/sgrif/yaqb/tree/master/yaqb_tests).

Struct annotations
------------------

### `#[derive(Queriable)]`

Adds an implementation of the [`Queriable`][queriable] trait to the annotated
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
annotations][#field-annotations] for additional configurations. If the struct
has a field for the primary key, an additional function, `save_changes(&mut
self, connection: &Connection) -> QueryResult<()>`, will be added to the model.
This will persist the model to the database and update it with any fields the
database returns.

[queriable]: http://sgrif.github.io/yaqb/yaqb/query_source/trait.Queriable.html
[insertable]: http://sgrif.github.io/yaqb/yaqb/trait.Insertable.html
[as_changeset]: http://sgrif.github.io/yaqb/yaqb/query_builder/trait.AsChangeset.html

Field annotations
-----------------

### `#[column_name="value"]`

Any field can be annotated with `column_name=` to have it map to a column with a
different name. This is required for all fields of tuple structs.
