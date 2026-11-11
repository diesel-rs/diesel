# An async interface for diesel

Diesel gets rid of the boilerplate for database interaction and eliminates
runtime errors without sacrificing performance. It takes full advantage of
Rust's type system to create a low overhead query builder that "feels like
Rust."

Diesel-async provides an async implementation of diesels connection implementation
and any method that may issue an query. It is designed as pure async drop-in replacement
for the corresponding diesel methods. Similar to diesel the crate is designed in a way 
that allows third party crates to extend the existing infrastructure and even provide 
their own connection implementations.

Supported databases:

1. PostgreSQL
2. MySQL
3. SQLite

## Usage 

### Simple usage

Diesel-async is designed to work in combination with diesel, not to replace diesel. For this it 
provides drop-in replacement for diesel functionality that actually interacts with the database.

A normal project should use a setup similar to the following one:

```toml
[dependencies]
diesel = "2.3.0" # no backend features need to be enabled
diesel-async = { version = "0.7.0", features = ["postgres"] }
```

This allows to import the relevant traits from both crates:

```rust
use diesel::prelude::*;
use diesel_async::{RunQueryDsl, AsyncConnection, AsyncPgConnection};

// ordinary diesel model setup

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: String,
}

// create an async connection
let mut connection = AsyncPgConnection::establish(&std::env::var("DATABASE_URL")?).await?;

// use ordinary diesel query dsl to construct your query
let data: Vec<User> = users::table
    .filter(users::id.gt(0))
    .or_filter(users::name.like("%Luke"))
    .select(User::as_select())
    // execute the query via the provided
    // async `diesel_async::RunQueryDsl`
    .load(&mut connection)
    .await?;
```

### Async Transaction Support

Diesel-async provides an ergonomic interface to wrap several statements into a shared 
database transaction. Such transactions are automatically rolled back as soon as 
the inner closure returns an error

``` rust
connection.transaction::<_, diesel::result::Error, _>(|conn| async move {
         diesel::insert_into(users::table)
             .values(users::name.eq("Ruby"))
             .execute(conn)
             .await?;

         let all_names = users::table.select(users::name).load::<String>(conn).await?;
         Ok(())
       }.scope_boxed()
    ).await?;
```

### Streaming Query Support

Beside loading data directly into a vector, diesel-async also supports returning a 
value stream for each query. This allows to process data from the database while they 
are still received.

```rust
// use ordinary diesel query dsl to construct your query
let data: impl Stream<Item = QueryResult<User>> = users::table
    .filter(users::id.gt(0))
    .or_filter(users::name.like("%Luke"))
    .select(User::as_select())
    // execute the query via the provided
    // async `diesel_async::RunQueryDsl`
    .load_stream(&mut connection)
    .await?;

```

### Built-in Connection Pooling Support

Diesel-async provides built-in support for several connection pooling crates. This includes support
for:

* [deadpool](https://crates.io/crates/deadpool)
* [bb8](https://crates.io/crates/bb8)
* [mobc](https://crates.io/crates/mobc)

#### Deadpool

``` rust
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::RunQueryDsl;

// create a new connection pool with the default config
let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(std::env::var("DATABASE_URL")?);
let pool = Pool::builder(config).build()?;

// checkout a connection from the pool
let mut conn = pool.get().await?;

// use the connection as ordinary diesel-async connection
let res = users::table.select(User::as_select()).load::(&mut conn).await?;
```

#### BB8

``` rust
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::RunQueryDsl;

// create a new connection pool with the default config
let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(std::env::var("DATABASE_URL")?);
let pool = Pool::builder().build(config).await?;

// checkout a connection from the pool
let mut conn = pool.get().await?;

// use the connection as ordinary diesel-async connection
let res = users::table.select(User::as_select()).load::(&mut conn).await?;
```

#### Mobc

``` rust
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::mobc::Pool;
use diesel_async::RunQueryDsl;

// create a new connection pool with the default config
let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(std::env::var("DATABASE_URL")?);
let pool = Pool::new(config);

// checkout a connection from the pool
let mut conn = pool.get().await?;

// use the connection as ordinary diesel-async connection
let res = users::table.select(User::as_select()).load::(&mut conn).await?;
```

## Diesel-Async with Secure Database

In the event of using this crate with a `sslmode=require` flag, it will be necessary to build a TLS cert.
There is an example provided for doing this using the `rustls` crate in the `postgres` examples folder.

## Crate Feature Flags

Diesel-async offers several configurable features:

* `postgres`: Enables the implementation of `AsyncPgConnection`
* `mysql`: Enables the implementation of `AsyncMysqlConnection`
* `deadpool`: Enables support for the `deadpool` connection pool implementation
* `bb8`: Enables support for the `bb8` connection pool implementation
* `mobc`: Enables support for the `mobc` connection pool implementation

By default no features are enabled.

## Code of conduct

Anyone who interacts with Diesel in any space, including but not limited to
this GitHub repository, must follow our [code of conduct](https://github.com/diesel-rs/diesel/blob/master/code_of_conduct.md).

## License

Licensed under either of these:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

### Contributing

Contributions are explicitly welcome. Please consider opening a [discussion](https://github.com/weiznich/diesel_async/discussions/categories/ideas)
with your idea first, to discuss possible designs.

Unless you explicitly state otherwise, any contribution you intentionally submit
for inclusion in the work, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.

