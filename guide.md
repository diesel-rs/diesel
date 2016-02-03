Getting Started 2.0
===

For this guide, we're going to walk through some simple examples for each of the
pieces of CRUD. Each step in this guide will build on the previous, and is meant
to be followed along. This guide assumes that you're using PostgreSQL on
Rust nightly. We'll talk about how to use Diesel on stable Rust in the final
chapter. Before we start, make sure you have PostgreSQL installed and running.

The first thing we need to do is generate our project.

```shell
cargo new diesel_demo
cd diesel_demo
```

We're going to use a [`.env`][dotenv-rust] file to manage our environment
variables for us, and [`diesel_cli`][diesel-cli] to help set up the database.

[dotenv-rust]: https://github.com/slapresta/rust-dotenv
[diesel-cli]: https://github.com/sgrif/diesel/tree/master/diesel_cli

```shell
echo DATABASE_URL=postgres://localhost/diesel_demo > .env
cargo install diesel_cli
diesel setup
```

This will create our database for us (if it didn't already exist), and create an
empty migrations directory that we can use to manage our schema. More on that
later. Let's add Diesel to our `Cargo.toml`. We'll also add `dotenv` so we can
use that same `.env` file for our environment variables.

```toml
[dependencies]
diesel = "^0.5.0"
diesel_codegen = { version = "^0.5.0", default-features = false, features = ["nightly", "postgres"] }
dotenv = "^0.8.0"
dotenv_macros = "^0.8.0"
```

We'll also add this to the top of `src/lib.rs`, which will allow us to use
features from `diesel_codegen`.

```rust
#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen, dotenv_macros)]
```

Now we're going to write a small CLI that lets us manage a blog (ignoring the
fact that we can only access the database from this CLI...). The first thing
we're going to need is a table to store our posts. Let's create a migration for
that:

```shell
diesel migration generate create_posts_table
```

You should see output which looks something like this:

```
Creating migrations/20160202154039_create_posts/up.sql
Creating migrations/20160202154039_create_posts/down.sql
```

Every migration consists of a way to run it, and a way to undo it. `down.sql`
should always leave your database in the state it was in before running `up.sql`
when possible. We'll edit our migration to contain the following:

```sql
-- up.sql
CREATE TABLE posts (
  id SERIAL PRIMARY KEY,
  title VARCHAR NOT NULL,
  body TEXT NOT NULL,
  published BOOLEAN NOT NULL DEFAULT 'f'
)

-- down.sql
DROP TABLE posts
```

We can run our new migration with `diesel migration run`. It's usually a good
idea to make sure that `down.sql` is correct. You can do a quick sanity check by
doing `diesel migration redo`.

OK enough SQL, let's write some Rust. We'll start by writing some code to show
us the last 5 published posts. The first thing we need to do is establish a
database connection.

```rust
// src/lib.rs
#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
```

We'll also want to create a `Post` struct that we can read our data into, and
have diesel generate the names we'll use to reference our tables and columns in
our queries.

We'll add the following two lines to `src/lib.rs`

```rust
pub mod schema;
pub mod models;
```

and then create those modules.

```rust
// src/models.rs
#[derive(Queryable)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

// src/schema.rs
infer_schema!(dotenv!("DATABASE_URL"));
```

The `#[derive(Queryable)]` will generate all of the code needed to load a `Post`
struct from a SQL query. The `infer_schema!` macro connects to the database URL
given to it, and creates a bunch of code based on the database schema to
represent all of the tables and columns. We'll see exactly what that looks like
next. Let's write the code to actually show us our posts.

```rust
// src/bin/show_posts.rs
extern crate diesel_demo;
extern crate diesel;

use self::diesel_demo::*;
use self::diesel_demo::models::*;
use self::diesel::prelude::*;

fn main() {
    use diesel_demo::schema::posts::dsl::*;

    let connection = establish_connection();
    let results = posts.filter(published.eq(true))
        .limit(5)
        .load(&connection)
        .expect("Error loading posts")
        .collect::<Vec<Post>>();

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("----------\n");
        println!("{}", post.body);
    }
}
```

The `use posts::dsl::*` line imports a bunch of aliases so that we can say
`posts` instead of `posts::table`, and `published` instead of
`posts::published`. It's useful when we're only dealing with a single table, but
that's not always what we want.

We can run our script with `cargo run --bin show_posts`. Unfortunately, the results
won't be terribly interesting, as we don't actually have any posts in the
database. Still, we've written a decent amount of code, so let's commit.

The full code for the demo at this point can be found [here][commit-no-1].

Next, let's write some code to create a new post. We'll want a struct to use for
inserting a new record.

```rust
// src/models.rs
use super::schema::posts;

#[insertable_into(posts)]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub body: &'a str,
}
```

Now let's add a function to save a new post.

```rust
// src/lib.rs
use self::models::{Post, NewPost};

pub fn create_post<'a>(conn: &PgConnection, title: &'a str, body: &'a str) -> Post {
    use schema::posts;

    let new_post = NewPost {
        title: title,
        body: body,
    };

    diesel::insert(&new_post).into(posts::table)
        .get_result(conn)
        .expect("Error saving new post")
}
```

When we call `.get_result` on an insert or update statement, it automatically
adds `RETURNING *` to the end of the query, and lets us load it into any struct
that implements `Queryable` for the right types. Neat!

#### Protip (This would be a sidebar or something)

Diesel can insert more than one record in a single query. Just pass a `Vec` or
slice to `insert`, and then call `get_results` instead of `get_result`. If you
don't actually want to do anything with the row that was just inserted, call
`.execute` instead. The compiler won't complain at you, that way. :)

Now that we've got everything set up, we can create a little script to write a
new post.

```rust
// src/bin/write_post.rs
extern crate diesel_demo;
extern crate diesel;

use self::diesel_demo::*;
use std::io::{stdin, Read};

fn main() {
    let connection = establish_connection();

    println!("What would you like your title to be?");
    let mut title = String::new();
    stdin().read_line(&mut title).unwrap();
    let title = &title[..(title.len() - 1)]; // Drop the newline character
    println!("\nOk! Let's write {} (Press {} when finished)\n", title, EOF);
    let mut body = String::new();
    stdin().read_to_string(&mut body).unwrap();

    let post = create_post(&connection, title, &body);
    println!("\nSaved draft {} with id {}", title, post.id);
}

#[cfg(not(windows))]
const EOF: &'static str = "CTRL+D";

#[cfg(windows)]
const EOF: &'static str = "CTRL+Z";
```

We can run our new script with `cargo run --bin write_post`. Go ahead and write
a blog post. Get creative! Here was mine:

```
   Compiling diesel_demo v0.1.0 (file:///Users/sean/Documents/Projects/open-source/diesel_demo)
     Running `target/debug/write_post`

What would you like your title to be?
Diesel demo

Ok! Let's write Diesel demo (Press CTRL+D when finished)

You know, a CLI application probably isn't the best interface for a blog demo.
But really I just wanted a semi-simple example, where I could focus on Diesel.
I didn't want to get bogged down in some web framework here.
Plus I don't really like the Rust web frameworks out there. We might make a
new one, soon.

Saved draft Diesel demo with id 1
```

Unfortunately, running `show_posts` still won't display our new post, because we
saved it as a draft. We need to publish it! But in order to do that, we'll need
to look at how to update an existing record. First, let's commit. The code for
this demo at this point can be found [here][commit-no-2].

Now that we've got create and read out of the way, update is actually relatively
simple. Let's jump write into the script:

```rust
// src/bin/publish_post.rs
extern crate diesel_demo;
extern crate diesel;

use self::diesel::prelude::*;
use self::diesel_demo::*;
use self::diesel_demo::models::Post;
use std::env::args;

fn main() {
    use diesel_demo::schema::posts::dsl::{posts, published};

    let id = args().nth(1).expect("publish_post requires a post id")
        .parse::<i32>().expect("Invalid ID");
    let connection = establish_connection();

    let post = diesel::update(posts.find(id))
        .set(published.eq(true))
        .get_result::<Post>(&connection)
        .expect(&format!("Unable to find post {}", id));
    println!("Published post {}", post.title);
}
```

That's it! Let's try it out with `cargo run --bin publish_post 1`.

```
   Compiling diesel_demo v0.1.0 (file:///Users/sean/Documents/Projects/open-source/diesel_demo)
     Running `target/debug/publish_post 1`
Published post Diesel demo
```

And now, finally, we can see our post with `cargo run --bin show_posts`.

```
     Running `target/debug/show_posts`
Displaying 1 posts
Diesel demo
----------

You know, a CLI application probably isn't the best interface for a blog demo.
But really I just wanted a semi-simple example, where I could focus on Diesel.
I didn't want to get bogged down in some web framework here.
Plus I don't really like the Rust web frameworks out there. We might make a
new one, soon.
```

We've still only covered 3 of the 4 letters of CRUD though. Let's show how to
delete things. Sometimes we write something we really hate, and we don't have
time to look up the ID. So let's delete based on the title, or even just some
words in the title.

```rust
// src/bin/delete_post.rs
extern crate diesel_demo;
extern crate diesel;

use self::diesel::prelude::*;
use self::diesel_demo::*;
use std::env::args;

fn main() {
    use diesel_demo::schema::posts::dsl::*;

    let target = args().nth(1).expect("Expected a target to match against");
    let pattern = format!("%{}%", target);

    let connection = establish_connection();
    let num_deleted = diesel::delete(posts.filter(title.like(pattern)))
        .execute(&connection)
        .expect("Error deleting posts");

    println!("Deleted {} posts", num_deleted);
}
```

We can run the script with `cargo run --bin delete_post demo` (at least with the
title I chose. Your output should look something like:

```rust
   Compiling diesel_demo v0.1.0 (file:///Users/sean/Documents/Projects/open-source/diesel_demo)
     Running `target/debug/delete_post demo`
Deleted 1 posts
```

When we try to run `cargo run --bin show_posts` again, we can see that the post
was in fact deleted. This barely scratches the surface of what you can do with
Diesel, but hopefully this tutorial has given you a good foundation to build off
of. We recommend exploring the [API docs](docs.diesel.rs) to see more. The final
code for this tutorial can be found [here][commit-no-3].

FIXME cover how to use syntex on stable, recommend using nightly for dev and
stable for deployment, show how to use both in the same codebase.

[commit-no-1]: FIXME ACTUAL LINK
[commit-no-2]: FIXME ACTUAL LINK
[commit-no-3]: FIXME ACTUAL LINK
