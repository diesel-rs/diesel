Deriving Traits in Depth
===============

Part of what makes Diesel's query builder so powerful is
its ability to assist writing safe SQL queries in Rust.
It enables this level of safety through implementing 
a series of traits on your structs.
Writing these implementations by hand can be very laborious,
so Diesel offers custom derives.

In this guide,
we will cover each trait in detail in terms of its use cases 
and usage considerations.
To be able to use these derives,
make sure you have `#[macro_use] extern crate diesel;` at the root of your project.

Throughout this guide,
we will be looking at examples for each trait individually and how they interact with each other.
Some of the example code will be implementing basic [CRUD] database operations.
We'll be covering creating, reading, and updating data.
The details of those operations will be not be covered beyond their relevance to the demonstrated trait.

In general, it may be more helpful to think of Diesel as a SQL query builder.
While Diesel does offer some standard [ORM] (Object Relation Mapper) features,
Diesel's code generation derives are for safely building SQL queries.

[ORM]: https://en.wikipedia.org/wiki/Object-relational_mapping
[CRUD]: https://en.wikipedia.org/wiki/Create,_read,_update_and_delete

- [Queryable](#queryable)
- [QueryableByName](#queryablebyname)
- [Insertable](#insertable)
- [Identifiable](#identifiable)
- [AsChangeset](#aschangeset)
- [Associations](#associations)

## Queryable
A `Queryable` struct is one that represents the 
data returned from a database query.
In many cases this may map exactly to the column structure of a single table,
however there may be cases where you need to make a query that spans several tables and/or only uses
a subset of columns. 
For this reason, it may be helpful to view `Queryable` structs as the *result* of your query.
It is acceptable and often desirable to have multiple `Queryable` structs for the same database table.

Annotating your struct with [`#[derive(Queryable)]`][queryable_doc] enables you to use Diesel's
[`RunQueryDsl`] to assist in retrieving data.
A few of the methods you may use are `load()`, `get_result()`, `get_results()`, and `first()`.
Should you make a query that doesn't return the same columns and values (in the order specified) on your `Queryable` struct,
you will get a compile-time error.
The only thing `Queryable` cares about is that the data returned from the query
maps exactly to your data structure.

[`RunQueryDsl`]: https://docs.diesel.rs/diesel/query_dsl/trait.RunQueryDsl.html
[queryable_doc]: https://docs.diesel.rs/diesel/deserialize/trait.Queryable.html

The following example shows making two different queries into the `users` table.
We get back a [`QueryResult`],
which is basically a wrapper around Rust's `Result` type.
That means we'll be able to use `expect()` to handle our error conditions.

[`QueryResult`]: https://docs.diesel.rs/diesel/result/type.QueryResult.html

### Example

```rust
// File: src/models.rs

use schema::users; // Brings the users table into scope

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Queryable)]
pub struct EmailUser {
    pub id: i32,
    pub email: String,
}
```

```rust
// File: src/main.rs
#[macro_use] extern crate diesel;

use diesel::prelude::*;

fn main() {
    // The following will return all users as a `QueryResult<Vec<User>>`
    let users_result: QueryResult<Vec<User>> = users.load(&db_connection);

    // Here we are getting the value (or error) out of the `QueryResult`
    // A successful value will be of type `Vec<User>`
    let users = users_result.expect("Error loading users");

    // Here, a successful value will be type `Vec<EmailUser>`
    let email_users = users.select((users::id, users::email))
        .load::<EmailUser>(&db_connection)
        .expect("Error loading the email only query");
}
```

If we were to comment out all three `String` fields on our `User` struct, we would see the following error.

```rust
error[E0277]: the trait bound `(i32,): diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text, diesel::sql_types::Text, diesel::sql_types::Text), _>` is not satisfied
  --> src/bin/main.rs:34:10
   |
34 |         .load::<User>(&db_conn)
   |          ^^^^ the trait `diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text, diesel::sql_types::Text, diesel::sql_types::Text), _>` is not implemented for `(i32,)`
   |
   = help: the following implementations were found:
             <(A,) as diesel::Queryable<(SA,), DB>>
   = note: required because of the requirements on the impl of `diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text, diesel::sql_types::Text, diesel::sql_types::Text), _>` for `diesel_demo::models::User`
   = note: required because of the requirements on the impl of `diesel::LoadQuery<_, diesel_demo::models::User>` for `diesel_demo::schema::users::table`
```

Notice the compiler is indicating a trait is not implemented for calling the `.load()` method.
When reading, take note of the values in the tuple(s).
(See Rust's [data type docs] if you're unfamiliar with tuples or their syntax.)

[data type docs]: https://doc.rust-lang.org/1.21.0/book/second-edition/ch03-02-data-types.html#grouping-values-into-tuples

> `diesel::Queryable<(diesel::sql_types::Integer, diesel::sql_types::Text, diesel::sql_types::Text, diesel::sql_types::Text), _>`


`Queryable` is trying to convert those four types into the types on our `User` struct.
Our struct has the three String fields commented out,
so it doesn't know what those `Text` columns are supposed to be converted to.
Remember, `Queryable` structs represent the exact columns, value, 
and ordering of your query's returned result,
which we are violating here.
`Queryable` is expecting a tuple that looks like `(i32, String, String, String)`,
but we currently only have a tuple consisting of `(i32,)`.
We need to add those `String` columns back for our code to compile again.

## QueryableByName
`Queryable` is the trait you normally use with Diesel's query builder.
If you're using the [`sql_query`] function,
you will instead need to implement the `QueryableByName` trait.

Diesel provides some escape hatches to execute raw SQL queries.
The problem with this is that Diesel can't ensure type safety
and it's accessing fields by name instead of by index.
This means that you can't deserialize the raw query result into a tuple or a regular struct.
Adding [`#[derive(QueryableByName)]`][queryable_by_name_doc] to your struct means 
that it will be able to be built from the result of a raw SQL query using the [`sql_query`] function.

[`sql_query`]: https://docs.diesel.rs/diesel/fn.sql_query.html
[queryable_by_name_doc]: https://docs.diesel.rs/diesel/deserialize/trait.QueryableByName.html

The implementation of `QueryableByName` assumes that each field on your struct
has a certain SQL type.
It makes these assumptions based on the annotations you add to your struct.
If your `QueryableByName` struct references a single table,
you may annotate that struct with `#[table_name="my_table"]`.
`QueryableByName` will bind the struct fields to the SQL types it finds in your table's schema.

You may also individually annotate each field on your struct
with `#[sql_type="ColumnTypeHere"]`.
If you're not using the `table_name` annotation,
every field in your struct needs to have this annotation.
When combining both `table_name` and `sql_type` annotations,
Diesel will override any fields using `sql_type` and pick the rest from the table.

`QueryableByName` also supports nested structs.
If you have any fields whose type is a struct that also implements `QueryableByName`,
you may add the `#[diesel(embed)]` annotation.

### Example 

```rust
// File: src/models.rs

// We're defining the posts and users tables here
// to illustrate the schema in the following example
table! {
    posts (id) {
        id -> Int4,
        user_id -> Int4,
        title -> Varchar,
        body -> Text,
    }
}

table! {
    users (id) {
        id -> Int4,
        first_name -> Varchar,
        last_name -> Varchar,
        email -> Varchar,
    }
}

#[derive(Debug, Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}


// Here we bring the Diesel types into scope
use diesel::sql_types::{Integer, Text};

// When not using the `table_name` annotation,
// we must tell `QueryableByName` the type of
// every field.

#[derive(Debug, QueryableByName)]
pub struct UserEmail {
    #[sql_type="Integer"]
    pub id: i32,
    #[sql_type="Text"]
    pub email: String,
}

// `full_name` is not a column on our users table,
// but we're planning on returning a column that
// concatenates the first and last name together.
// `QueryableByName` will use the users table for
// all other column types.
#[derive(Debug, QueryableByName)]
#[table_name="users"]
pub struct UserName {
    pub first_name: String,
    pub last_name: String,
    #[sql_type="Text"]
    pub full_name: String,
}

// The user_name field maps to another
// `QueryableByName` struct and all other fields
// reference the posts table.
#[derive(Debug, QueryableByName)]
#[table_name="posts"]
pub struct PostsWithUserName {
    #[diesel(embed)]
    pub user_name: UserName,
    pub title: String,
    pub body: String,
}

```

```rust
// File: src/main.rs
#[macro_use] extern crate diesel;

use diesel::prelude::*;
use diesel::sql_query;

fn main() {
    diesel::insert_into(users::table)
        .values(
          (
            users::first_name.eq("Gordon"),
            users::last_name.eq("Freeman"),
            users::email.eq("gordon.freeman@blackmesa.co"),
          )
        )
        .execute(&db_connection)
        .expect("Error inserting row into database");

    let first_user = users
        .first::<User>(&db_connection)
        .expect("Error querying first user");

    diesel::insert_into(posts::table)
        .values(
          (
            posts::user_id.eq(first_user.id),
            posts::title.eq("Thoughts on Tomorrow's Experiment"),
            posts::body.eq("What could possibly go wrong?"),
          )
        )
        .execute(&db_connection)
        .expect("Error inserting row into database");

    let users_emails = sql_query("SELECT users.id, users.email FROM users ORDER BY id")
      .load::<UserEmail>(&connection);

    println!("{:?}", users_emails); 
    //=> User { id: 1, email: "gordon.freeman@blackmesa.co" }
    
    let joined = sql_query("
      SELECT users.first_name, 
             users.last_name,
             CONCAT(users.first_name, users.last_name) as full_name,
             posts.body,
             posts.title 
      FROM users 
      INNER JOIN posts ON users.id = posts.user_id
    ")
      .load::<PostsWithUserName>(&connection);
    println!("{:?}", joined); 
    /* Output =>
        [
            PostsWithUserName { 
                user_name: UserName { first_name: "Gordon", last_name: "Freeman", full_name: "GordonFreeman" }, 
                title: "Thoughts on Tomorrow's Experiment",
                body: "What could possibly go wrong? 
            }
        ]
    */
}
```

If we were to forget one of `QueryableByName`'s required struct annotations,
we would see the following error.

```rust
error: Your struct must either be annotated with `#[table_name = "foo"]` or have all of its fields annotated with `#[sql_type = "Integer"]`
 --> src/models.rs:4:17
  |
4 | #[derive(Debug, QueryableByName)]
  |                 ^^^^^^^^^^^^^^^
```

If we were to comment out the `sql_type` annotation for `UserName`'s `full_name` field,
we would see the following error.

```rust
error[E0412]: cannot find type `full_name` in module `users`
  --> src/models.rs:12:17
   |
12 | #[derive(Debug, QueryableByName)]
   |                 ^^^^^^^^^^^^^^^ not found in `users`
```

## Insertable

The simplest way to insert data in Diesel is by working with tuples.
However, this can become tedious when inserting a lot of data,
for example if it were a large web form deserialized by a library like Serde.
The `Insertable` trait is meant to be implemented on structs
whose data you want to easily insert into your database.
To implement `Insertable` on your struct,
add the [`#[derive(Insertable)]`][insertable_doc] annotation.

As with usual Diesel inserts, 
you will still be using the [`.insert_into()`]
method to generate a SQL `INSERT` statement for that table.
You may chain [`.values()`] or [`.default_values()`]
to add the values for that `INSERT` statement.
In addition to tuples,
[`.values()`] also accepts a reference to an `Insertable` struct for inserting a single record.

You may also pass [`.values()`] a `&Vec<T>` or a `&[T]` to insert multiple records at once.
On backends that support the `DEFAULT` keyword,
the data will be inserted in a single query.
On SQLite, one query will be performed per row.

[`.insert_into()`]: https://docs.diesel.rs/diesel/fn.insert_into.html
[`.values()`]: https://docs.diesel.rs/diesel/query_builder/struct.IncompleteInsertStatement.html#method.values
[`.default_values()`]: https://docs.diesel.rs/diesel/query_builder/struct.IncompleteInsertStatement.html#method.default_values
[insertable_doc]: https://docs.diesel.rs/diesel/prelude/trait.Insertable.html

For `Insertable` structs, Diesel needs to know the corresponding table name.
You must add the `#[table_name="some_table_name"]` attribute to your `Insertable` struct.
If your struct has different field names than the columns they reference,
they may be annotated with `#[column_name = "some_column_name"]`.

Typically, you will not use `Queryable` and `Insertable` together.
Thinking of web forms again, a new record wouldn't have such fields as
`id`, `created_at`, or `updated_at`.

### Example

```rust
// File: src/models.rs

// Add serde, serde_derive, and serde_json to simulate 
// deserializing a web form.
extern crate serde;
extern crate serde_derive;
extern crate serde_json;


use schema::users;

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Deserialize, Insertable)]
#[table_name="users"]
pub struct NewUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    #[column_name = "email"]
    pub electronic_mail: &'a str,
 }
```

```rust
// File: src/main.rs
#[macro_use] extern crate diesel;

use diesel::prelude::*;

fn main() {
    // Here we simulate a webform by deserializing a json string
    // into a NewUser struct.
    let new_user: NewUser = serde_json::from_str(
        r#"{
            "first_name": "Gordon",
            "last_name": "Freeman",
            "electronic_mail": "gordon.freeman@blackmesa.co"
        }"#).unwrap();

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&db_connection)
        .expect("Error inserting row into database");

    let all_users = users.load::<User>(&db_connection)
        .expect("Error loading users");

    println!("User count: {}", all_users.len());
    //=> User count: 1
}
```

If we try to insert records without deriving `Insertable`, we would get the following error.

```rust
error[E0277]: the trait bound `&diesel_demo::models::NewUser<'_>: diesel::Insertable<diesel_demo::schema::users::table>` is not satisfied
  --> src/bin/main.rs:29:10
   |
29 |         .values(&new_user)
   |          ^^^^^^ the trait `diesel::Insertable<diesel_demo::schema::users::table>` is not implemented for `&diesel_demo::models::NewUser<'_>`
```

The compiler reports that `Insertable` isn't implemented for our `NewUser` struct.

## Identifiable

Certain database operations,
such as table [associations](#associations) and updates, 
require that rows be uniquely identifiable.
Implementing the `Identifiable` trait on your struct will define which columns (primary keys)
make your struct uniquely identifiable.
To implement `Identifiable`, 
annotate your struct with [`#[derive(Identifiable)]`][identifiable_doc].

[identifiable_doc]: https://docs.diesel.rs/diesel/associations/trait.Identifiable.html

By default, `Identifiable` will assume the primary key is a column named `id`.
If your table's primary key is named differently,
you can annotate the table with the attribute `#[primary_key(some_field_name)` or `#[primary_key(some_field_name, another_field_name)`.
The `Identifiable` trait will assume that the annotated struct will be named in the singular form of the table it corresponds to.
If the table name differs you may use the `#[table_name="some_table_name"]` attribute annotation.
The `Identifiable` trait gives us the `id()` method on our models,
which returns the value of our record's primary key.

In the following example,
First, we'll look at some of the behavior `Identifiable` gives us.
After that, let's add the annotation to our `User` struct.
Finally, we will then attempt to get the value of the first record's primary key by calling `id()` and also update the first and last name of our user.

### Example

```rust
// File: src/models.rs

use schema::users;

#[derive(Identifiable, Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: &'a str,
 }
```

```rust
// File: src/main.rs
#[macro_use] extern crate diesel;

use diesel::prelude::*;

fn main() {
    let new_user = NewUser { 
        first_name: "Gordon", 
        last_name: "Freeman", 
        electronic_mail: "gordon.freeman@blackmesa.co",
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&db_connection)
        .expect("Error inserting row into database");

    let all_users = users.load::<User>(&db_connection)
        .expect("Error loading users");

    println!("User count: {}", all_users.len());
    //=> User count: 1

    let hero = users.first::<Users>(&db_connection)
        .expect("Error loading first user");

    println!("Our Hero's ID: {}", hero.id());
    //=> Our Hero's ID: 1

    diesel::update(&hero).set((
        first_name.eq("Alyx"),
        last_name.eq("Vance"),
    )).execute(&db_connection);
    
    let updated_hero = users.first::<Users>(&db_connection)
        .expect("Error loading first user");
    
    println!("Our Hero's updated name: {} {}", updated_hero.first_name, updated_hero.last_name);
    //=> Our Hero's updated name: Alyx Vance
}
```

If we were to try and call `id()` and also update our record without implementing `Identifiable`, we would get the following errors.

```rust
error[E0599]: no method named `id` found for type `diesel_demo::models::User` in the current scope
  --> src/bin/main.rs:35:40
   |
35 |     println!("Our Hero's Id: {}", hero.id());
   |                                        ^^ field, not a method
   |
   = help: did you mean to write `hero.id` instead of `hero.id(...)`?

error[E0277]: the trait bound `&diesel_demo::models::User: diesel::Identifiable` is not satisfied
  --> src/bin/main.rs:37:5
   |
37 |     diesel::update(&hero).set(
   |     ^^^^^^^^^^^^^^ the trait `diesel::Identifiable` is not implemented for `&diesel_demo::models::User`
   |
   = note: required because of the requirements on the impl of `diesel::query_builder::IntoUpdateTarget` for `&diesel_demo::models::User`
   = note: required by `diesel::update`

error[E0277]: the trait bound `diesel_demo::models::User: diesel::associations::HasTable` is not satisfied
  --> src/bin/main.rs:37:5
   |
37 |     diesel::update(&hero).set(
   |     ^^^^^^^^^^^^^^^^^^^^^ the trait `diesel::associations::HasTable` is not implemented for `diesel_demo::models::User`
   |
   = note: required because of the requirements on the impl of `diesel::associations::HasTable` for `&diesel_demo::models::User`

continued stack trace...
```

The first error shows that the compiler is expecting to find a field `id` on the struct
even though we wanted the method.
The second error explicitly states the `Identifiable` trait is not implemented for our `User` struct.

The compiler is also giving us some more hints about trait bounds in each one of those *note* sections.
The error messages regarding `IntoUpdateTarget` and `HasTable` are due to the trait bounds
on the `update()` method that we called after `id()`.

## AsChangeset

As we've seen with inserting data, updating data with tuples can also become tedious.
`AsChangeset` serves a similar purpose to `Insertable` in that
it gives us a way to easily update a large amount of deserialized data.
To derive the `AsChangeset` trait, add
[`#[derive(AsChangeset)]`][aschangeset_doc] to your struct.

In this section we will change our schema to make our `email` field nullable.
`AsChangeset` allows us to deal with updating nullable fields a few different ways,
which are changing the value, nulling the value, or ignoring the field.

The nullable fields on our structs will now be of type `Option<T>`.
Usually you do not want to change the primary key of the row or rows that you're updating.
For this reason, we don't want to have `AsChangeset` and `Queryable` annotated on the same struct,
which means we'll need another struct for updating our records.

[aschangeset_doc]: https://docs.diesel.rs/diesel/query_builder/trait.AsChangeset.html

Before we dive into some examples,
let us take a look at our new schema.

```rust
// Output of "diesel print-schema"

table! {
    users (id) {
        id -> Int4,
        first_name -> Varchar,
        last_name -> Varchar,
        email -> Nullable<Varchar>,
    }
}
```

`AsChangeset` will automatically assume you don't want to change your record's primary key,
so it will ignore that column.
If your primary key field name is different than `id`,
you must annotate your struct with the `#[primary_key(your_key)]` attribute.

By default, `AsChangeset` will assume that anytime a field has the value `None`,
we do not want to assign any values to it (ignore).
If we truly want to assign a `NULL` value,
we can use the annotation `#[changeset_options(treat_none_as_null="true")]`.
Be careful, as when you are setting your `Option<T>` fields to `None`,
they will be `NULL` in the database instead of ignored.

However, there is a way to have both `AsChangeset` behaviors on a single struct.
Instead of using the field type `Option<T>`,
you may use `Option<Option<T>>`.
When updating, a value of `None` will be ignored and a value of `Some(None)` will `NULL` that column.
All three options are shown in the following example code.
Notice some of the code uses `unwrap_or()`:
The values being returned are `None`,
but we want to see output on the screen.
Passing in a `String` will let us print out something to the screen
like "this is a nulled field!".

### Example

```rust
// File: src/models.rs

use schema::users;

#[derive(Identifiable, Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<&'a str>,
 }

// This struct will ignore any fields with the value None
// and NULL any fields with the value Some(None) - all the behaviors we need.
#[derive(AsChangeset)]
#[table_name="users"]
pub struct IgnoreNoneFieldsUpdateUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<Option<&'a str>>,
 }

// This struct will set the column to NULL if its value is None
// Notice the treat_none_as_null option.
#[derive(AsChangeset)]
#[changeset_options(treat_none_as_null="true")]
#[table_name="users"]
pub struct NullNoneFieldsUpdateUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<&'a str>,
 }
```

```rust
// File: src/main.rs
#[macro_use] extern crate diesel;

use diesel::prelude::*;

fn main() {
    let new_user = NewUser { 
        first_name: "Gordon", 
        last_name: "Freeman", 
        email: "gordon.freeman@blackmesa.co",
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&db_connection)
        .expect("Error inserting row into database");

    let all_users = users.load::<User>(&db_connection)
        .expect("Error loading users");

    println!("User count: {}", all_users.len());
    //=> User count: 1

    // Querying User
    let hero = users.first::<User>(&db_connection)
        .expect("Error loading first user");

    // Update scenario 1
    let ignore_fields_update = IgnoreNoneFieldsUpdateUser {
        first_name: "Issac",
        last_name: "Kleiner",
        email: None, // Field to be ignored when updating
    }

    diesel::update(&hero).set(&ignore_fields_update)
        .execute(&db_connection);
    
    let updated_hero = users.first(&db_connection)::<Users>
        .expect("Error loading first user");

    println!("Name: {} {} Email: {}", updated_hero.first_name, 
        updated_hero.last_name, 
        updated_hero.email.unwrap(),
    );

    // Output
    //=> Name: Issac Kleiner Email: gordon.freeman@blackmesa.ca 
    
    // Update scenario 2 
    let null_a_field_update = IgnoreNoneFieldsUpdateUser {
        first_name: "Issac",
        last_name: "Kleiner",
        email: Some(None), // Nulls the column in the DB
    }

    diesel::update(&hero).set(&null_a_field_update)
        .execute(&db_connection);
    
    let updated_hero = users.first::<User>(&db_connection)
        .expect("Error loading first user");

    println!("Name: {} {} Email: {}", updated_hero.first_name, 
        updated_hero.last_name, 
        updated_hero.email.unwrap_or("This field is now Nulled".to_string()),
    );

    // Output
    //=> Name: Issac Kleiner Email: This field is now Nulled

    // Update scenario 3
    let null_fields_update = NullNoneFieldsUpdateUser {
        first_name: "Eli",
        last_name: "Vance",
        email: None, // with option treat_none_as_null=true
    }

    diesel::update(&hero).set(&null_fields_update)
        .execute(&db_connection);
    
    let updated_hero = users.first::<User>(&db_connection)
        .expect("Error loading first user");

    println!("Name: {} {} Email: {:?}", updated_hero.first_name, 
        updated_hero.last_name, 
        updated_hero.email.unwrap_or("This is a Null value".to_string()),
    );

    // Output
    //=> Name: Eli Vance Email: This is a Null value
}
```

Suppose we comment out the AsChangeset derive for one of our update struct. Let's look at the compiler error we get.

```rust
error[E0277]: the trait bound `&diesel_demo_cli::models::IgnoreFieldsUpdateUser<'_>: diesel::query_builder::AsChangeset` is not satisfied
  --> src/bin/main.rs:47:33
   |
47 |     diesel::update(&hero).set(&ignore_fields_update)
   |                                 ^^^ the trait `diesel::query_builder::AsChangeset` is not implemented for `&diesel_demo_cli::models::IgnoreFieldsUpdateUser<'_>`
```

Here we get a clear message from Rust indicating that we're trying to use this 
struct without an implementation of `AsChangeset`.

## Associations

Diesel makes querying for related records very easy via the `Associations` trait.
All data relations are uni-directional and focus on the *child to parent* relationship
between two database tables.
One example of such a relation would be a `User` *has-many* `Posts`.
In this case, the parent is `User` and the child is `Posts`.

To implement `Associations`, on the child struct
use the [`#[derive(Associations)]`][associations_doc] annotation.
You will also need the `#[belongs_to(ParentStruct)]` annotation to reference its parent.

[associations_doc]: https://docs.diesel.rs/diesel/associations/index.html

Both parent and child structs must have the [Identifiable](#identifiable) trait implemented.
By default, the foreign key column on the child should take the form of `parent_id`.
If there is a custom foreign key,
the `belongs_to` attribute must be written as 
`#[belongs_to(ParentStruct, foreign_key="my_custom_key")]`.

Let's take a look at how we would set up the relation between a User and their Posts.

### Example 

```rust
// Output of "diesel print-schema"

table! {
    posts (id) {
        id -> Int4,
        user_id -> Int4,
        title -> Varchar,
        content -> Varchar,
    }
}

table! {
    users (id) {
        id -> Int4,
        first_name -> Varchar,
        last_name -> Varchar,
        email -> Nullable<Varchar>,
    }
}
```

```rust
// File: src/models.rs

use schema::{posts, users};

#[derive(Identifiable, Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<&'a str>,
 }

#[derive(AsChangeset)]
#[table_name="users"]
pub struct UpdateUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<Option<&'a str>>,
 }

// Setting up the association to users
#[derive(Identifiable, Associations, Queryable)]
#[belongs_to(User)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub content: String,
}

// Lets us insert new posts
#[derive(Insertable)]
#[table_name="posts"]
pub struct NewPost<'a> {
    pub user_id: i32,
    pub title: &'a str,
    pub content: &'a str,
 }
```

```rust
// File: src/main.rs
#[macro_use] extern crate diesel;

use diesel::prelude::*;

fn main() {
    let new_user = NewUser { 
        first_name: "Issac", 
        last_name: "Kleiner", 
        email: Some("issac.kleiner@blackmesa.co"),
    };

    let issac_kleiner = diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(&db_connection)
        .expect("Error inserting row into database");

    // Setting up our posts vec and pointing each post to the 
    // most recently inserted user
    let post_list = vec![
        NewPost {
            user_id: issac_kleiner.id().to_owned(),
            title: "Top Secret #001",
            content: "I'm feeling optimistic about our new reactor code written in Rust!",
        },
        NewPost {
            user_id: issac_kleiner.id().to_owned(),
            title: "Top Secret #002",
            content: "Finished making special pet for Gordon. I hope he likes it!",
        },
    ];

    // Insert the new posts vector
    diesel::insert_into(posts::table)
        .values(&post_list)
        .execute(&db_connection)
        .expect("Error inserting post");

    // Get the first user
    let issac = users.first::<User>(&db_connection)
        .expect("Couldn't find first user");

    // Get all the posts belonging to the first user
    let issacs_posts = Post::belonging_to(&issac)
        .get_results::<Post>(&db_connection)
        .expect("Couldn't find associated posts");

    for post in issacs_posts {
        println!("-----\nTitle: {}\nContent: {}\n", post.title, post.body);
    }

  // Outputs
  /*
------
Title: Top Secret #001
Content: "I'm feeling optimistic about our new reactor code written in Rust!"

------
Title: Top Secret #002
Content: "Finished making special pet for Gordon. I hope he likes it!"

  */
}
```

If we were to forget to derive `Associations` on `Posts`,
we would see the following error.

```rust
error[E0599]: no function or associated item named `belonging_to` found for type `diesel_demo::models::Post` in the current scope
   --> src/bin/main.rs:103:24
    |
103 |     let issacs_posts = Post::belonging_to(&issac)
    |                        ^^^^^^^^^^^^^^^^^^ function or associated item not found in `diesel_demo::models::Post`
```

## Conclusion

Please check out other [official guides] and [docs]
for more information on using the Diesel framework.

If you have any questions, join our [gitter channel],
the Diesel team is happy to help.

[official guides]: https://diesel.rs/guides/
[docs]: https://docs.diesel.rs
[gitter channel]: https://gitter.im/diesel-rs/diesel
