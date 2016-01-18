Diesel - A safe, extensible ORM and Query Builder for Rust
==========================================================

[![Build Status](https://travis-ci.org/sgrif/diesel.svg)](https://travis-ci.org/sgrif/diesel)
[Documentation](http://sgrif.github.io/diesel/diesel/index.html)

Diesel gets rid of the boilerplate for database interaction and eliminates
runtime errors, without sacrificing performance. It takes full advantage of
Rust's type system to create a low overhead query builder that "feels like
Rust".

We are not feature complete, nor do I think we've covered all use cases. If
you've found something difficult to accomplish, please open an issue.

Getting Started
---------------

Before you can do anything, you'll first need to set up your table. You'll want
to specify the columns and tables that exist using the [`table!` macro][table]
Once you've done that, you can already start using the query builder, and
pulling out primitives.

Much of the behavior in diesel comes from traits, and it is recommended that you
import `diesel::prelude::*`. We avoid exporting generic type names, or any bare
functions at that level.

```rust
#[macro_use]
extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
        favorite_color -> Nullable<VarChar>,
    }
}

fn users_with_name(connection: &Connection, target_name: &str)
    -> QueryResult<Vec<(i32, String, Option<String>)>>
{
    use self::users::dsl::*;
    users.filter(name.eq(target_name)).load(connection)
        .map(|x| x.collect())
}
```

Note that we're importing `users::dsl::*` here. This allows us to deal with
only the users table, and not have to qualify everything. If we did not have
this import, we'd need to put `users::` before each column, and reference the
table as `users::table`.

You can also use
[diesel_codegen](https://github.com/sgrif/diesel/tree/master/diesel_codegen) to
call [`table!`][table] for you automatically, based on your existing database
schema. See the [diesel_codegen
README](https://github.com/sgrif/diesel/tree/master/diesel_codegen) for details
on how to get started. Here's the same code with Diesel Codegen.

```rust
#[macro_use] extern crate diesel;

use diesel::prelude::*;

infer_schema!(dotenv!("DATABASE_URL"));

fn users_with_name(connection: &Connection, target_name: &str)
    -> QueryResult<Vec<(i32, String, Option<String>)>>
{
    use self::users::dsl::*;
    users.filter(name.eq(target_name)).load(connection)
        .map(|x| x.collect())
}
```

If you want to be able to query for a struct, you'll need to implement the
[`Queryable` trait][queryable] Luckily,
[diesel_codegen](https://github.com/sgrif/diesel/tree/master/diesel_codegen) can do
this for us automatically.

```rust
#[derive(Queryable, Debug)]
pub struct User {
    id: i32,
    name: String,
    favorite_color: Option<String>,
}

fn main() {
    let connection = Connection::establish(env!("DATABASE_URL"))
        .unwrap();
    let users: Vec<User> = users::table.load(&connection)
        .unwrap().collect();

    println!("Here are all the users in our database: {:?}", users);
}
```

Database Migrations
-------------------

Diesel CLI is a tool that aids in managing your database schema. Migrations are
bi-directional changes to your database that get applied sequentially. You can
use it on your project like so:

```shell
cargo install diesel_cli
mkdir migrations
diesel migration generate create_users_table
```

You'll see that two files were generated for you,
`migrations/{current_timestamp}_create_users_table/up.sql` and
`migrations/{current_timestamp}_create_users_table/down.sql`. You should edit
these files to show how to update your schema, and how to undo that change.

```sql
-- up.sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    favorite_color VARCHAR
);
```

```sql
-- down.sql
DROP TABLE USERS;
```

You can then run your new migration by running `diesel migration run`. Make sure
that you set the `DATABASE_URL` environment variable first, or pass it directly
by doing `diesel migration run --database-url="postgres://localhost/your_database"`
Alternatively, you can call
[`diesel::migrations::run_pending_migrations`][pending-migrations] from
`build.rs`.

Diesel will automatically keep track of which migrations have already been run,
ensuring that they're never run twice.

[pending-migrations]: http://sgrif.github.io/diesel/diesel/migrations/fn.run_pending_migrations.html

If you ever need to revert or make changes to your migrations, `diesel
migration revert` will revert the last migration run, and `diesel migration
redo` will revert and then rerun the last migration run. Type `diesel
migration --help` for more information.

Insert
------

Inserting data requires implementing the [`Insertable` trait][insertable]. Once
again, we can have this be automatically implemented for us by the compiler.

```rust
#[insertable_into(users)]
struct NewUser<'a> {
    name: &'a str,
    favorite_color: Option<&'a str>,
}

fn create_user(connection: &Connection, name: &str, favorite_color: Option<&str>)
  -> QueryResult<User>
{
    let new_user = NewUser {
        name: name,
        favorite_color: favorite_color,
    };
    diesel::insert(&new_user).into(users::table).get_result(connection)
}
```

[`insert`][insert] can return any struct which implements
[`Queryable`][queryable] for the right type. If you don't actually want to use
the results, you should call [`execute`][execute]
instead, or the compiler will complain that it can't infer what type you meant
to return. You can use the same struct for inserting and querying if you'd like,
but you'll need to make columns that are not present during the insert optional
(e.g. `id` and timestamps). For this reason, you probably want to create a new
struct instead.

You might notice that we're having to manually grab the first record that was
inserted. This is because [`insert`][insert] can also take a slice or `Vec` of
records, and will insert them in a single query. For this reason,
[`insert`][insert] will always return an `Iterator`. A helper for this common
case will likely be added in the future.

For both `#[derive(Queryable)]` and `#[insertable_into]`, you can annotate any
single field with `#[column_name="name"]`, if the name of your field differs
from the name of the column. This annotation is required on all fields of tuple
structs. This cannot be used, however, to work around name collisions with
keywords that are reserved in Rust, as you cannot have a column with that name.
This may change in the future.

```rust
#[insertable_into(users)]
struct NewUser<'a>(
    #[column_name="name"]
    &'a str,
    #[column_name="favorite_color"]
    Option<&'a str>,
)

fn create_user(connection: &Connection, name: &str, favorite_color: Option<&str>)
  -> QueryResult<User>
{
    let new_user = NewUser(name, favorite_color);
    diesel::insert(&new_user).into(users::table).get_result(connection)
}
```

Update
------

To update a record, you'll need to call the [`update`][update] function.
Here's a simple example.

```rust
fn change_users_name(connection: &Connection, target: i32, new_name: &str) -> QueryResult<User> {
    use users::dsl::*;

    diesel::update(users.filter(id.eq(target))).set(name.eq(new_name))
        .get_result(&connection)
}
```

As with [`insert`][insert], we can return any type which implements
[`Queryable`][queryable] for the right types. If you do not want to use the
returned record(s), you should call [`execute`][execute] instead of
[`get_result`][get_result] or [`get_results`][get_results].

You can also use a struct to represent the changes, if it implements
[`AsChangeset`][as_changeset]. Again, `diesel_codegen` can generate this for us
automatically.

```rust
#[changeset_for(users)]
pub struct UserChanges {
    name: String,
    favorite_color: Option<String>,
}

fn save_user(connection: &Connection, id: i32, changes: &UserChanges) -> QueryResult<User> {
    diesel::update(users::table.filter(users::id.eq(id))).set(changes)
        .get_result(connection)
}
```

Note that even though we've implemented [`AsChangeset`][as_changeset], we still
need to specify what records we want to update. If the struct has the primary
key on it, a method called `save_changes` will also be added.

```rust
#[changeset_for(users)]
pub struct User {
    id: i32,
    name: String,
    favorite_color: Option<String>,
}

fn change_name_to_jim(connection: &Connection, user: &mut User) -> QueryResult<()> {
    user.name = "Jim".into();
    user.save_changes(connection)
}
```

This method will update the model with any fields that are updated in the
database (for example, if you have timestamps which are updated by triggers).

Delete
------

[`delete`][delete] works very similarly to [`update`][delete], but does not
support returning a record.

```rust
fn delete_user(connection: &Connection, user: User) -> QueryResult<()> {
    use users::dsl::*;

    let deleted_rows = try!(diesel::delete(users.filter(id.eq(user.id))).execute(connection));
    debug_assert!(deleted_rows == 1);
    Ok(())
}
```

How do I do other things?
-------------------------

Take a look at the various files named on what you're trying to do in
https://github.com/sgrif/diesel/tree/master/diesel_tests/tests. See
https://github.com/sgrif/diesel/blob/master/diesel_tests/tests/schema.rs for how
you can go about getting the data structures set up.

[as_changeset]: http://sgrif.github.io/diesel/diesel/query_builder/trait.AsChangeset.html
[connection]: http://sgrif.github.io/diesel/diesel/struct.Connection.html
[delete]: http://sgrif.github.io/diesel/diesel/query_builder/fn.delete.html
[execute]: http://sgrif.github.io/diesel/diesel/trait.ExecuteDsl.html#method.execute
[get_result]: http://sgrif.github.io/diesel/diesel/prelude/trait.LoadDsl.html#method.get_result
[get_results]: http://sgrif.github.io/diesel/diesel/prelude/trait.LoadDsl.html#method.get_results
[insert]: http://sgrif.github.io/diesel/diesel/query_builder/fn.insert.html
[insertable]: http://sgrif.github.io/diesel/diesel/trait.Insertable.html
[queryable]: http://sgrif.github.io/diesel/diesel/query_source/trait.Queryable.html
[table]: http://sgrif.github.io/diesel/diesel/macro.table!.html
[update]: http://sgrif.github.io/diesel/diesel/query_builder/fn.update.html

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
