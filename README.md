YAQB (This will not be the real name. Please open PRs with name ideas)
======================================================================

NOTE: You need to be on nightly >= 11/27/2015. Sorry. I hope to work on stable soon
===================================================================================

This is an early stage ORM in Rust. It is poorly documented, and rapidly
iterating. I would love early feedback on usage. Help in documenting current
usage would also be welcomed.

The goal is to take a different approach here. This is not a port of Active
Record or Hibernate. This is an attempt to find what a "Rust ORM" is. So far,
what that seems to be is something that is statically guaranteed to only allow
correct queries, while still feeling high level.

An "incorrect query" includes, but is not limited to:

- Invalid SQL syntax
- Attempting to interpret a column as the wrong type (e.g. reading varchar as
  i32, treating a nullable column as something other than an option)
- Selecting a column from another table
- Selecting columns that are not used (this doesn't mean that you have to access
  that field on your struct, but the struct must have that field)

Does it support X?
------------------

0.1 progress is tracked on https://github.com/sgrif/yaqb/issues/1

Getting Started
---------------

Before you can do anything, you'll first need to set up your table You'll want
to specify the columns and tables that exist using the
[`table!` macro](https://github.com/sgrif/yaqb/blob/master/yaqb/src/macros.rs#L45).
Once you've done that, you can already start using the query builder, and
pulling out primitives.

Much of the behavior in yaqb comes from traits, and it is recommended that you
import `yaqb::*`. We avoid exporting generic type names, or any bare functions
at that level.

```rust
#[macro_use]
extern crate yaqb;

use yaqb::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
        favorite_color -> Nullable<VarChar>,
    }
}

fn users_with_name(connection: &Connection, target_name: &str)
    -> Vec<(i32, String, Option<String>)>
{
    use self::users::dsl::*;
    users.filter(name.eq(target_name))
        .load(connection)
        .unwrap()
        .collect()
}
```

Note that we're importing `users::dsl::*` here. This allows us to deal with only
the users table, and not have to qualify everything. If we did not have this
import, we'd need to put `users::` before each column, and reference the table
as `users::table`.

If you want to be able to query for a struct, you'll need to implement the
[`Queriable` trait](https://github.com/sgrif/yaqb/blob/master/yaqb/src/query_source/mod.rs#L11).
Luckily, [yaqb_codegen](https://github.com/sgrif/yaqb/tree/master/yaqb_codegen)
can do this for us automatically.

```rust
#[derive(Queriable, Debug)]
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

Insert
------

Inserting data requires implementing the
[`Insertable` trait](https://github.com/sgrif/yaqb/blob/master/yaqb/src/persistable.rs#L8).
Once again, we can have this be automatically implemented for us by the
compiler.

```rust
#[insert_into=users]
struct NewUser<'a> {
    name: &'a str,
    favorite_color: Option<&'a str>,
}

fn create_user(connection: &Connection, name: &str, favorite_color: Option<&str>)
  -> DbResult<User>
{
    let new_user = NewUser {
        name: name,
        favorite_color: favorite_color,
    };
    connection.insert(&users::table, &new_user)
        .map(|mut result| result.nth(0).unwrap())
}
```

`insert` can return any struct which implements `Queriable` for the right type.
If you don't actually want to use the results, you should call
`insert_returning_count` instead, or the compiler will complain that it can't
infer what type you meant to return. You use the same struct for inserting and
querying if you'd like, but you'll need to make the `id` and columns such as
timestamps optional when they otherwise wouldn't be. For this reason, you
probably want to create a new struct intead.

You might notice that we're having to manually grab the first record that was
inserted. That is because `insert` can also take a slice or `Vec` of records,
and will insert them in a single query. For this reason, `insert` will always
return an `Iterator`. A helper for this common case will likely be added in the
future.

For both `#[derive(Queriable)]` and `#[insertable_into]`, you can annotate any
single field with `#[column_name="name"]`, if the name of your field differs
from the name of the column. This annotation is required on all fields of tuple
structs. This cannot be used, however, to work around name collisions with
keywords that are reserved in Rust, as you cannot have a column with that name.
This may change in the future.

```rust
#[insert_into=users]
struct NewUser<'a>(
    #[column_name="name"]
    &'a str,
    #[column_name="favorite_color"]
    Option<&'a str>,
)

fn create_user(connection: &Connection, name: &str, favorite_color: Option<&str>)
  -> DbResult<User>
{
    let new_user = NewUser(name, favorite_color);
    connection.insert(&users::table, &new_user)
        .map(|mut result| result.nth(0).unwrap())
}
```

Update
------

To update a record, you'll need to call the `update` function. Unlike `insert`
(which may change to use this pattern in the future), `update` is a top level
function which creates a query that you'll later pass to the `Connection`.
Here's a simple example.

```rust
fn change_users_name(connection: &Connection, target: i32, new_name: &str) -> DbResult<User> {
    use yaqb::query_builder::update;
    use users::dsl::*;

    let command = update(users.filter(id.eq(target))).set(name.eq(new_name));
    connection.query_one(&command)
        .map(|r| r.unwrap())
}
```

Similar to `insert`, we always return a `Result<Option<Model>>`, as we can't
tell at compile time if this is the kind of query that always returns at least 1
result. This may change in the future.

As with `insert`, we can return any type which implements `Queriable` for the
right types. If you do not want to use the returned record(s), you should call
`execute_returning_count` instead of `query_one` or `query_all`.

You can also use a struct to represent the changes, if it implements
`AsChangeset`. You can generate that from a macro (FIXME: This should be a
compiler annotation not long after the time of writing this. If it is later than
12/5/15, please open an issue as I'm being lazy)

```rust
changeset! {
    User => users {
        name -> String,
        favorite_color -> Option<String>,
    }
}

fn save_user(connection: &Connection, user: &mut User) -> DbResult<()> {
    let command = update(users::table.filter(users::id.eq(user.id)))
        .set(user);
    let updated_user = try!(connection.query_one(&command)).unwrap();
    *user = updated_user;
    Ok(())
}
```

Note that even though we've implemented `AsChangeset`, we still need to specify
what records we want to update. There will likely be changes that make it harder
to accidentally update the entire table before 1.0.

Delete
------

Delete works very similarly to `update`, but does not support returning a
record.

```rust
fn delete_user(connection: &Connection, user: User) -> DbResult<()> {
    use yaqb::query_builder::delete;
    use users::dsl::*;

    let command = delete(users.filter(id.eq(user.id)));
    let deleted_rows = try!(connection.execute_returning_count(&command));
    debug_assert!(deleted_rows == 1);
    Ok(())
}
```

FIXME: Replace links to source code with hosted doc pages

How do I do other things?
-------------------------

Take a look at the various files named on what you're trying to do in
https://github.com/sgrif/yaqb/tree/master/yaqb_tests/tests. See
https://github.com/sgrif/yaqb/blob/master/yaqb_tests/tests/schema.rs for how
you can go about getting the data structures set up.
