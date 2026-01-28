# A benchmark suite for relational database connection crates in rust

This directory contains a basic benchmark suite that allows to compare different crates that can be used to connect to relational database systems. Those benchmarks are created with the following goals:

a) To track diesels performance and find potential regressions
b) To evaluate potential alternatives for the currently used C-dependencies
c) To compare diesel with the competing crates

It currently supports the following database systems:

* PostgreSQL
* MySQL/MariaDB
* SQLite

and the following crates:

* Diesel
* [SQLx](https://github.com/launchbadge/sqlx)
* [Rustorm](https://github.com/ivanceras/rustorm)
* [Quaint](https://github.com/prisma/quaint)
* [Postgres](https://github.com/sfackler/rust-postgres)
* [Rusqlite](https://github.com/rusqlite/rusqlite)
* [Mysql](https://github.com/blackbeam/rust-mysql-simple)
* [diesel-async](https://github.com/weiznich/diesel_async)
* [wtx](https://github.com/c410-f3r/wtx)

By default only diesels own benchmarks are executed. To run the benchmark do the following:

```sh
$ DATABASE_URL=your://database/url diesel migration run --migration-dir ../migrations/$backend
$ DATABASE_URL=your://database/url cargo bench --features "$backend"
```

To enable other crates add the following features:

* `SQLx: ` `sqlx-bench sqlx/$backend $backend`
* `Rustorm`: `rustorm rustorm/with-$backend rustorm_dao $backend`
* `SeaORM`: `sea-orm sea-orm/sqlx-$backend sqlx tokio criterion/tokio futures $backend`
* `Quaint`: `quaint quaint/$backend tokio quaint/serde-support serde $backend`
* `Postgres`: `rust_postgres $backend`
* `Rusqlite`: `rusqlite $backend`
* `Mysql`: `rust-mysql $backend`
* `diesel-async`: `diesel-async diesel-async/$backend $backend tokio`
* `wtx`: `$backend tokio/rt-multi-thread wtx`

## Benchmarks

### Common data structures

#### Table definitions

The following schema definition was used. (For Mysql/Sqlite postgres specific types where replaced by their equivalent type).

```sql
CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  hair_color VARCHAR
);

CREATE TABLE posts (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL,
  title VARCHAR NOT NULL,
  body TEXT
);

CREATE TABLE comments (
  id SERIAL PRIMARY KEY,
  post_id INTEGER NOT NULL,
  text TEXT NOT NULL
);
```

#### Struct definitions

```rust
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}

pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: Option<String>,
}

pub struct Comment {
    id: i32,
    post_id: i32,
    text: String,
}
```

Field types are allowed to differ, to whatever type is expected compatible by the corresponding crate.

### `bench_trivial_query`

This benchmark tests how entities from a single table are loaded. Before starting the benchmark 1, 10, 100, 1000 or 10000 entries are inserted into the `users` table. For this the `id` of the user is provided by the autoincrementing id, the `name` is set to `User {id}` and the `hair_color` is set to `Null`. An implementation of this benchmark is expected to return a list of the type `User.

### `bench_medium_complex_query`

This benchmark tests how entities from more than one table are loaded. Before starting the benchmark 1, 10, 1000, 10000 entries are inserted into the `users` table.  For this the `id` of the user is provided by the autoincrementing id, the `name` is set to `User {id}` and the `hair_color` is set to `"black"` for even id's, to `"brown"` otherwise. An implementation of this benchmark is expected to return a list of the type `(User, Option<Post>)` filtered by `hair_color = "black"` so that matching pairs of `User` and `Option<Post>` are returned. Though the `posts` table is empty the corresponding implementation needs to query both tables. 

### `bench_insert`

This benchmark tests how fast entities are inserted into the database. For this each implementation gets a size how many entries are scheduled to be inserted into the database. An implementation of this benchmark is expected to insert as many entries into the user table as the number provided by the benchmark framework. It is not required to clean up the already inserted entries at any time.  Newly inserted users are generated using the following rules: `id` of the user is provided by the autoincrementing id, the `name` is set to `User {batch_id}` and the `hair_color` is set to `"hair_color"`.

### `bench_loading_associations_sequentially`

This benchmark tests how fast a complex set of entities is received from the database. 
Before starting the benchmark a large amount of data needs to be inserted into the database. The `users` table is required to contain 100 entries (or for sqlite 9 entries) based on the following rules: `id` is determined by the autoincrementing column, `name` is set to `User {batch_id}` and `hair_color` is set to `"black"` for even id's, otherwise to `"brown"`. For each entry in the `users` table, 10 entries in the `posts` table need to exist. Each entry in the `posts` table is based on the following rules: `id` is autogenerated by autoincrementing column, `title` is set to `Post {post_batch_id} for user {user_id}` where `post_batch_id` referees to number of the current post in relation to the user (so between 0 and 9), `user_id` is set to the corresponding user's id, and `body` is set to `NULL`. For each entry in the `posts` table 10 entries in the `comments` table are generated based on the following rules:
`id` is set to the autoincrementing default value, `text` is set to `Comment {comment_batch_id} on post {post_id}` where `comment_batch_id` referees to the number of the current comment in relation to the post (so between 0 and 9), `post_id` is set tho the corresponding posts's id. 
An implementation of the benchmark is expected to return a list of the type `(User, Vec<(Post, Vec<Comment>)>)`, that contains all users wit all corresponding comments and posts grouped by their corresponding associations.
