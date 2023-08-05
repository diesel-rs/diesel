[![diesel logo](https://diesel.rs/assets/images/diesel_logo_stacked_black.png)](https://diesel.rs)

# A safe, extensible ORM and Query Builder for Rust

[![Build Status](https://github.com/diesel-rs/diesel/workflows/CI%20Tests/badge.svg)](https://github.com/diesel-rs/diesel/actions?query=workflow%3A%22CI+Tests%22+branch%3Amaster)
[![Gitter](https://badges.gitter.im/diesel-rs/diesel.svg)](https://gitter.im/diesel-rs/diesel?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
[![Crates.io](https://img.shields.io/crates/v/diesel.svg)](https://crates.io/crates/diesel)

API Documentation: [latest release](https://docs.rs/diesel) – [master branch](https://docs.diesel.rs/master/diesel/index.html)

[Homepage](https://diesel.rs)

Diesel gets rid of the boilerplate for database interaction and eliminates
runtime errors without sacrificing performance. It takes full advantage of
Rust's type system to create a low overhead query builder that "feels like
Rust."

Supported databases:
1. [PostgreSQL](https://docs.diesel.rs/master/diesel/pg/index.html)
2. [MySQL](https://docs.diesel.rs/master/diesel/mysql/index.html)
3. [SQLite](https://docs.diesel.rs/master/diesel/sqlite/index.html)

You can configure the database backend in `Cargo.toml`:

```toml
[dependencies]
diesel = { version = "<version>", features = ["<postgres|mysql|sqlite>"] }
```

## Getting Started

Find our extensive Getting Started tutorial at
[https://diesel.rs/guides/getting-started](https://diesel.rs/guides/getting-started).
Guides on more specific features are coming soon.

## Getting help

If you run into problems, Diesel has a very active Gitter room.
You can come ask for help at
[gitter.im/diesel-rs/diesel](https://gitter.im/diesel-rs/diesel).
For help with longer questions and discussion about the future of Diesel,
open a discussion on [GitHub Discussions](https://github.com/diesel-rs/diesel/discussions).

## Usage

### Simple queries

Simple queries are a complete breeze. Loading all users from a database:

```rust
users::table.load(&mut connection)
```

Executed SQL:

```sql
SELECT * FROM users;
```

Loading all the posts for a user:

``` rust
Post::belonging_to(user).load(&mut connection)
```

Executed SQL:

```sql
SELECT * FROM posts WHERE user_id = 1;
```

### Complex queries

Diesel's powerful query builder helps you construct queries as simple or complex as
you need, at zero cost.

```rust
let versions = Version::belonging_to(krate)
  .select(id)
  .order(num.desc())
  .limit(5);
let downloads = version_downloads
  .filter(date.gt(now - 90.days()))
  .filter(version_id.eq_any(versions))
  .order(date)
  .load::<Download>(&mut conn)?;
```

Executed SQL:

```sql
SELECT version_downloads.*
  WHERE date > (NOW() - '90 days')
    AND version_id = ANY(
      SELECT id FROM versions
        WHERE crate_id = 1
        ORDER BY num DESC
        LIMIT 5
    )
  ORDER BY date
```

### Less boilerplate

Diesel codegen generates boilerplate for you. It lets you focus on your business logic, not mapping to and from SQL rows.

That means you can write this:

```rust
#[derive(Queryable, Selectable)]
#[diesel(table_name = downloads)]
pub struct Download {
    id: i32,
    version_id: i32,
    downloads: i32,
    counted: i32,
    date: SystemTime,
}
```

Instead of this without Diesel:

```rust
pub struct Download {
    id: i32,
    version_id: i32,
    downloads: i32,
    counted: i32,
    date: SystemTime,
}

impl Download {
    fn from_row(row: &Row) -> Download {
        Download {
            id: row.get("id"),
            version_id: row.get("version_id"),
            downloads: row.get("downloads"),
            counted: row.get("counted"),
            date: row.get("date"),
        }
    }
}
```

### Inserting data

It's not just about reading data. Diesel makes it easy to use structs for new records.

```rust
#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser<'a> {
    name: &'a str,
    hair_color: Option<&'a str>,
}

let new_users = vec![
    NewUser { name: "Sean", hair_color: Some("Black") },
    NewUser { name: "Gordon", hair_color: None },
];

insert_into(users)
    .values(&new_users)
    .execute(&mut connection);
```

Executed SQL:

```sql
INSERT INTO users (name, hair_color) VALUES
  ('Sean', 'Black'),
  ('Gordon', DEFAULT)
```

If you need data from the rows you inserted, just change `execute` to `get_result` or `get_results`. Diesel will take care of the rest.

```rust
let new_users = vec![
    NewUser { name: "Sean", hair_color: Some("Black") },
    NewUser { name: "Gordon", hair_color: None },
];

let inserted_users = insert_into(users)
    .values(&new_users)
    .get_results::<User>(&mut connection);
```

Executed SQL:

```sql
INSERT INTO users (name, hair_color) VALUES
  ('Sean', 'Black'),
  ('Gordon', DEFAULT)
  RETURNING *
```

### Updating data

Diesel's codegen can generate several ways to update a row, letting you encapsulate your logic in the way that makes sense for your app.

Modifying a struct:

```rust
post.published = true;
post.save_changes(&mut connection);
```

One-off batch changes:

```rust
update(users.filter(email.like("%@spammer.com")))
    .set(banned.eq(true))
    .execute(&mut connection)
```

Using a struct for encapsulation:

```rust
update(Settings::belonging_to(current_user))
    .set(&settings_form)
    .execute(&mut connection)
```

### Raw SQL

There will always be certain queries that are just easier to write as raw SQL, or can't be expressed with the query builder. Even in these cases, Diesel provides an easy to use API for writing raw SQL.

```rust
#[derive(QueryableByName)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: String,
    organization_id: i32,
}

// Using `include_str!` allows us to keep the SQL in a
// separate file, where our editor can give us SQL specific
// syntax highlighting.
sql_query(include_str!("complex_users_by_organization.sql"))
    .bind::<Integer, _>(organization_id)
    .bind::<BigInt, _>(offset)
    .bind::<BigInt, _>(limit)
    .load::<User>(&mut conn)?;
```

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

Before contributing, please read the [contributors guide](https://github.com/diesel-rs/diesel/blob/master/CONTRIBUTING.md)
for useful information about setting up Diesel locally, coding style and common abbreviations.

Unless you explicitly state otherwise, any contribution you intentionally submit
for inclusion in the work, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.
