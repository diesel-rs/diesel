All About Updates
=================

Most applications fall into a category called "CRUD" apps. CRUD stands for "Create, Read, Update, Delete". Diesel provides support for all four pieces, but in this guide we're going to look at all the different ways to go about updating records.

An update statement is constructed by calling `diesel::update(target).set(changes)`. The resulting statement is then run by calling either `execute`, `get_result`, or `get_results`.

If you look at the documentation for [`update`](http://docs.diesel.rs/diesel/fn.update.html), you'll notice that the type of the argument is any type `T` which implements `IntoUpdateTarget`. You don't need to worry about what this trait does, but it is important to know which types implement it. There are three kinds which implement this trait. The first is tables.

If we have a table that looks like this:

```rust
table! {
    posts {
        id -> BigInt,
        title -> Text,
        body -> Text,
        draft -> Bool,
        publish_at -> Timestamp,
        visit_count -> Integer,
    }
}
```

We could write a query that publishes all posts by doing

```rust
use posts::dsl:::*;

diesel::update(posts).set(draft.eq(false))
```

We can look use the [`print_sql!`] macro to inspect the generated SQL. The macro's output will differ slightly from what actually gets sent to the database. It shows backend agnostic SQL, so the syntax used for bind parameters and quoting table/column names will be slightly different. If we look at the SQL from the above query, we'll see the following.

[`print_sql!`]: http://docs.diesel.rs/diesel/macro.print_sql.html

```sql
UPDATE `posts` SET `draft` = ?
```

This is pretty much one-to-one with the Rust code (the `?` denotes a bound parameter in SQL, which will be substituted with `false` here). It's quite rare to want to update an entire table, though. So let's look at how we can scope that down. The second kind that you can pass to `update` is any query which has only had `.filter` called on it. We could scope our update to only touch posts where `publish_at` is in the past like so:

```rust
use posts::dsl::*;
use diesel::expression::dsl::now;

let target = posts.filter(publish_at.lt(now));
diesel::update(target).set(draft.eq(false))
```

That would generate the following SQL

```sql
UPDATE `posts` SET `draft` = ? WHERE `posts`.`publish_at` < CURRENT_TIMESTAMP
```

The most common update queries are just scoped to a single record. So the final kind that you can pass to `update` is anything which implements [the `Identifiable` trait]. `Identifiable` gets implemented by putting `#[derive(Identifiable)]` on a struct. It represents any struct which is one-to-one with a row on a database table.

[the `Identifiable` trait]: http://docs.diesel.rs/diesel/associations/trait.Identifiable.html

If we wanted a struct that mapped to our posts table, it'd look something like this:

```rust
#[derive(Identifiable)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub draft: bool,
    pub publish_at: SystemTime,
}
```

The struct has one field per database column, but what's important for `Identifiable` is that it has the `id` field, which is the primary key of our table. Since our struct name is just the table name without an `s`, we don't have to provide the table name explicitly. If our struct were named something different, or if pluralizing it was more complex than putting an `s` on the end, we would have to specify the table name by adding `#[table_name="posts"]`. We're using `SystemTime` here since it's in the standard library, but in a real application we'd probably want to use a more full-featured type like one from `chrono`, which you can do by enabling the `chrono` feature on Diesel.

If we wanted to publish just this post, we could do it like this:

```rust
diesel::update(&post).set(posts::draft.eq(false))
```

It's important to note that we always pass a reference to the post, not the post itself. When we write `update(post)`, that's equivalent to writing `update(posts.find(post.id))`, or `update(posts.filter(id.eq(post.id)))`. We can see this in the generated SQL:

```sql
UPDATE `posts` SET `draft` = ? WHERE `posts`.`id` = ?
```

Now that we've seen all the ways to specify what we want to update, let's look at the different ways to provide the data to update it with. We've already seen the first way, which is to pass `column.eq(value)` directly. So far we've just been passing Rust values here, but we can actually use any Diesel expression. For example, we could increment a column:

```rust
use posts::dsl::*;

diesel::update(posts).set(visit_count.eq(visit_count + 1))
```

That would generate this SQL:

```sql
UPDATE `posts` SET `visit_count` = `posts`.`visit_count` + 1
```

Note that to use the Rust `+` operator on our column, we'll need to have written `numeric_expr!(posts::visit_count)` somewhere after we declared it. (Diesel may do this automatically for you in the future.) Assigning values directly is great for small, simple changes. If we wanted to update multiple columns this way, we can pass a tuple.

```rust
use posts::dsl::*;

diesel::update(posts)
  .set((
      title.eq("[REDACTED]"),
      body.eq("This post has been classified"),
  ))
```

This will generate exactly the SQL you'd expect

```sql
UPDATE `posts` SET `title` = ?, `body` = ?
```

While it's nice to have the ability to update columns directly like this, it can quickly get cumbersome when dealing with forms that have more than a handful of fields. If we look at the signature of [`.set`], you'll notice that the constraint is for a trait called [`AsChangeset`]. This is another trait that `diesel_codegen` can derive for us. We can add `#[derive(AsChangeset)]` to our `Post` struct, which will let us pass a `&Post` to `set`.

[`.set`]: http://docs.diesel.rs/diesel/query_builder/struct.IncompleteUpdateStatement.html#method.set
[`AsChangeset`]: http://docs.diesel.rs/diesel/query_builder/trait.AsChangeset.html

```rust
diesel::update(posts::table).set(&post)
```

The SQL will set every field present on the `Post` struct *except* for the primary key.

```sql
UPDATE `posts` SET
    `title` = ?,
    `body` = ?,
    `draft` = ?,
    `publish_at` = ?,
    `visit_count` = ?
```

Changing the primary key of an existing row is almost never something that you want to do, so `#[derive(AsChangeset)]` assumes that you want to ignore it. The only way to change the primary key is to explicitly do it with `.set(id.eq(new_id))`. However, note that `#[derive(AsChangeset)]` doesn't have the information from your table definition. If the primary key is something other than `id`, you'll need to put `#[primary_key(your_primary_key)]` on the struct as well.

If the struct has any optional fields on it, these will also have special behavior. By default, `#[derive(AsChangeset)]` will assume that `None` means that you don't wish to assign that field. For example, if we had the following code:

```rust
#[derive(AsChangeset)]
#[table_name="posts"]
struct PostForm<'a> {
    title: Option<&'a str>,
    body: Option<&'a str>,
}

diesel::update(posts::table)
    .set(&PostForm {
        title: None,
        body: Some("My new post"),
    })
```

That would generate the following SQL:

```sql
UPDATE `posts` SET `body` = ?
```

If you wanted to assign `NULL` instead, you can either specify `#[changeset_options(treat_none_as_null="true")]` on the struct, or you can have the field be of type `Option<Option<T>>`. Diesel doesn't currently provide a way to explicitly assign a field to its default value, though it may be provided in the future.

<!-- TODO: Cover `execute` vs `get_result` vs `get_results` vs `save_changes` -->
<!-- TODO: Briefly mention upsert exists, point to docs -->
