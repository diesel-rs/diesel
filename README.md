YAQB (This will not be the real name)
=====================================

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

How do I use it?
----------------

Take a look at the various files named on what you're trying to do in
https://github.com/sgrif/yaqb/tree/master/yaqb_tests/tests. See
https://github.com/sgrif/yaqb/blob/master/yaqb_tests/tests/schema.rs for how
you can go about getting the data structures set up.

Getting Started
---------------

Before you can do anything, you'll first need to set up your table You'll want
to specify the columns and tables that exist using the
[`table!` macro](https://github.com/sgrif/yaqb/blob/master/yaqb/src/macros.rs#L45).
Once you've done that, you can already start using the query builder, and
pulling out primitives.

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
    users.filter(name.eq(target_name))
        .load(connection)
        .unwrap()
        .collect()
}
```

If you want to be able to query for a struct, you'll need to implement the
[`Queriable` trait](https://github.com/sgrif/yaqb/blob/master/yaqb/src/query_source/mod.rs#L11).
Luckily, [yaqb_codegen](https://github.com/sgrif/yaqb/tree/master/yaqb_codegen)
can do this for us automatically.

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

#[derive(Queriable, Debug)]
pub struct User {
    id: i32,
    name: String,
    favorite_color: Option<String>,
}

fn main() {
    let connection = Connection::establish(env!("DATABASE_URL"))
        .unwrap();
    let users: Vec<User> = users.load(&connection).unwrap().collect();

    println!("Here are all the users in our database: {:?}", users);
}
```

FIXME: Replace links to source code with hosted doc pages
