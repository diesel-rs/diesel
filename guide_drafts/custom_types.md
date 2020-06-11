# Custom types in Diesel

Recently, in a personal project, I decided to use [diesel](http://diesel.rs/), an ORM for Rust language that supports some popular relational databases. My hands-on experience with ORM was quite limited at that time, and I first made an SQL schema without even analysing for supported features. Unfortunately, my beliefs in seamless type integration between an app and schema were broken quite soon. Consider the following SQL schema:

```sql
CREATE TYPE Language AS ENUM (
    'en', 'de', 'ru'
);

CREATE TABLE translations (
    word_id        INTEGER NOT NULL,
    translation_id INTEGER NOT NULL,
    language       Language NOT NULL,

    PRIMARY KEY (word_id, translation_id)
)
```

And after `diesel migration run` the following _schema.rs_ will be generated:

```rust
// src/schema.rs

table! {
    translations (word_id, translation_id) {
        word_id -> Int4,
        translation_id -> Int4,
        language -> Language,
    }
}
```

Great, I thought back then. There I can simply define an enum and ORM will do all of the deriving magic. As you may guess the actual answer is no and that is completely fair because database handles types differently (and sqlite does not even support them). So let's do all the manual work to be able to use it.

The first thing is to define the type in rust that can be used in the schema. That is quite straightforward:

```rust
// src/model.rs

pub enum Language {
    En,
    De,
    Ru,
}

pub mod exports {
    // we will use that a bit later
    pub use super::Language;
}
```

Thereafter we need to include this type to the schema. If you directly add `use crate::model::Language` to the source there will be 2 issues. Firstly it would be overridden on the next migration run. The second is newly appeared errors regarding diesel types, like this:

```
error[E0412]: cannot find type `Int4` in this scope
   --> src/schema.rs:5:20
    |
5   |           word_id -> Int4,
    |                      ^^^^ help: a trait with a similar name exists: `Into`
```

The last issue appears due to `table!` macro implementation. `use diesel::sql_types::*` is implicitly included if no other includes found. That might be handy if diesel exported with a different name or you want to change the behaviour of default types. So in our case types module usage should be explicitly included as well. The first issue can be handled via _diesel.toml_ configuration for diesel_cli tool like this:

```toml
[print_schema]
file = "src/schema.rs"
import_types = ["diesel::sql_types::*", "crate::model::exports::*"]
```

`import_types` here is simply the list of `use`-s that are going to be included at the beginning of each generated `table!` in the schema. If you follow the step-by-step, ensure that your schema file is regenerated with `diesel migration redo`. At this point you should still have compiler error with a message like this:

```
the trait `diesel::sql_types::NotNull` is not implemented for `model::Language`
```

And here comes the magic of diesel typing. As doc states `NotNull` is a marker trait that marks an SQL type in diesel terms. Instead of manual implementation `#[derive(SqlType)]` can be used, which also implements some other internal traits. At this point, the dummy project should compile without any problems. That is quite a nice feature of diesel to allow working with tables without implementing necessary conversions if the custom type is unused.

So let's write some conversions, so we can work with insertions and queries.
First of all, diesel differentiates an SQL type and the particular kind of used structure. In that architecture, a single SQL type `INTEGER` might be used for different "high-level" user types, like `bool`, `u16`, `i32`, etc.
Thus, we need to create a separate type `LanguageType` and use it in a generated schema, besides there should be a relation between our "high-level" type and SQL type.

```rust
#[derive(SqlType)]
#[postgres(type_name = "Language")]
pub struct LanguageType;

#[derive(Debug, FromSqlRow, AsExpression)]
#[sql_type = "LanguageType"]
pub enum Language {
    En,
    Ru,
    De,
}

pub mod exports {
    pub use super::LanguageType as Language;
}
```

`#[postgres(type_name = "..")]` provides a name of the type as declared in our SQL, similarly `#[mysql_type = ".."]` can be used for MySql and `#[sqlite_type = ".."]` for sqlite. `FromSqlRow` derives internal types that are necessary for querying and `AsExpression` for querying. `#[sql_type = "LanguageType"]` creates a relation between a type marker and a "high-level" type for insertions as well.

We are ready to implement real conversions. The first one is `ToSql`, that performs conversion to the bytes. Since it can convert our type to SQL, it allows to only make queries.

```rust
use std::io::Write;

use diesel::backend::Backend;
use diesel::serialize::{self, IsNull, Output, ToSql};

impl<Db: Backend> ToSql<LanguageType, Db> for Language {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Db>) -> serialize::Result {
        match *self {
            Language::En => out.write_all(b"en")?,
            Language::Ru => out.write_all(b"ru")?,
            Language::De => out.write_all(b"de")?,
        }
        Ok(IsNull::No)
    }
}
```

That's a little bit verbose, though pretty straightforward. We just write down bytes to trait provided output, that's it. Here may be anything that the underlying database can support. So if you want to write some complicated type instead of an enum, the one should consider using a concrete backend (e.g. `diesel::pg::Pg`). And here's the implementation of `FromSql`.

```rust
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;

impl FromSql<LanguageType, Pg> for Language {
    fn from_sql(bytes: Option<&<Pg as Backend>::RawValue>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"en" => Ok(Language::En),
            b"ru" => Ok(Language::Ru),
            b"de" => Ok(Language::De),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
```

Notice here the usage of specific backend type. It is mandatory since you need somehow to handle a generic `Backend::RawValue` (that has no trait bounds), for `Pg` in particular, it is `&[u8]` (at the diesel 1.4.4, in current master that is changed to `diesel::pg::PgValue` wrapper). Code here should be also self-explanatory though still a bit verbose.

That's it! Or at least almost. That would be quite unfair to ignore a crate that generates all that stuff (though only for enums) that we dig into at this post — [diesel-derive-enum](https://crates.io/crates/diesel-derive-enum). So all of the described conversions can be generated simply with:

```rust
use diesel_derive_enum::DbEnum;

#[derive(Debug, PartialEq, DbEnum)]
pub enum Language {
    En,
    Ru,
    De,
}
```

You can find complete sources used in this post [here](https://github.com/l4l/diesel-custom-types) and leave a comment on [reddit](https://www.reddit.com/r/rust/comments/gptvej/custom_types_in_diesel/).
