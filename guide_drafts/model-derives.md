Model Derives in Depth
===============

Diesel lets you write safe queries by using traits that you can implement on your models.
Writing these implementations by hand can be very laborious, so in addition to the `diesel` crate, the `diesel_codegen` crate offers custom derives.
In this section, we will cover each trait in detail in terms of its use cases, usage considerations,
as well compare hand written implementations to its derive.
To make use of these derives, `diesel_codegen` must be imported into the root of your project.

Throughout this guide, we will be looking at examples for each trait and how they interact together.
Some of the example code will be implementing basic [CRUD] database operations.
We'll be covering creating, reading, and updating data.
The details of those operations will be not be covered beyond their relevance to the demonstrated trait.

[CRUD]: https://en.wikipedia.org/wiki/Create,_read,_update_and_delete

- [Queryable](#queryable)
- [Insertable](#insertable)
- [Identifiable](#identifiable)
- [AsChangeset](#aschangeset)
- [Associations](#associations)

## <a name="queryable"></a>Queryable
Using the [`#[derive(Queryable)]`][queryable_doc] trait annotation on your model struct allows records to be queried from the database.
This means that any time you are using the `load()`, `get_result()`, `get_results()`, and `first()` methods to execute your queries,
you must have `Queryable` implemented on your model.
Queryable doesn't directly provide these methods,
but the traits these methods come from the `FirstDsl` and `LoadDsl` portions of the `diesel::prelude`,
which require your type to implement `Queryable`.

[queryable_doc]: http://docs.diesel.rs/diesel/query_source/trait.Queryable.html

`Queryable` structs do not necessarily have to be 1:1 with the table in your database.
You may have some queries where you only need to select a subset of columns.
For these cases, creating another struct and annotating it with `Queryable` will be sufficient.

The following example shows making two different queries into the `users` table.
We get back a [`QueryResult`],
which is basically a wrapper around the rust `Result` type.
That means we'll get the same familar api and can use `expect()` to handle our error conditions.

[`QueryResult`]: http://docs.diesel.rs/diesel/result/type.QueryResult.html

### Example 

*Note: src/lib.rs with #[macro_use] extern crate diesel_codegen; not shown*

*src/models.rs*
```rust
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

*src/main.rs*
```rust
// externs, database connection, and using statement code omitted

fn main() {
    // The following will return all Users as a QueryResult<Vec<User>>
    let users_result: QueryResult<Vec<User>> = users.load(&db_connection);

    // Here we are getting the value (or error) out of the QueryResult
    // A successful value will be of type Vec<User>
    let users = users_result.expect("Error loading users");

    // Here, a successful value will be type Vec<EmailUser> 
    let email_users = users.select((users::id, users::email))
        .load::<EmailUser>(&db_connection)
        .expect("Error loading the email only query");
}
```

If we were to comment out all the three `String` fields on our `User` struct, we would see the following error.

```rust
error[E0277]: the trait bound `(i32,): diesel::types::FromSqlRow<(diesel::types::Integer, diesel::types::Text, diesel::types::Text, diesel::types::Text), _>` is not satisfied

  --> src/bin/main.rs:21:28
   |
21 |     let query_result = users.load::<User>(&db_connection).expect("Error loading users");
   |                              ^^^^ the trait `diesel::types::FromSqlRow<(diesel::types::Integer, diesel::types::Text, diesel::types::Text, diesel::types::Text), _>` is not implemented for `(i32,)`
   |
   = help: the following implementations were found:
             <(A,) as diesel::types::FromSqlRow<(SA,), DB>>
   = note: required because of the requirements on the impl of `diesel::Queryable<(diesel::types::Integer, diesel::types::Text, diesel::types::Text, diesel::types::Text), _>` for `diesel_demo_cli::models::User`
   = note: required because of the requirements on the impl of `diesel::LoadQuery<_, diesel_demo_cli::models::User>` for `diesel_demo_cli::schema::users::table`
```

Notice the compiler is indicating a trait is not implmented for calling the `.load()` method.
When reading, take note of the values in the tuple[s].
> `FromSqlRow<(Integer, Text, Text, Text, Text)>` not implemented for `(i32,`).


`FromSqlRow` is trying to convert those 4 types into the types on our `User` model struct.
Our model struct has the three String fields commented out, so it doesn't know what those `Text` columns are supposed to be converted to.
In other words,
`FromSqlRow` is expecting a tuple that looks like `(i32, String, String, String)`,
but we currently only have a tuple consisting of `(i32,)`. We need to add those `String` columns back for our code to compile again.

This trait bound is illustrated below in the "hand-written" derive example.

```rust

// Implementing the Queryable trait by hand

use diesel::query_source::Queryable;
use diesel::backend::Backend;
use diesel::types::{Integer, Text, HasSqlType, FromSqlRow};

pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

impl<ST, DB> Queryable<ST, DB> for User where
    DB: Backend + HasSqlType<ST>,
    (i32, String, String, String): FromSqlRow<ST, DB> {
    
    type Row = (i32, String, String, String);

    fn build(row: &Self::Row) -> Self {
        User {
            id: row.0,
            first_name: row.1,
            last_name: row.2,
            email: row.3,
        }
    }
}
```

The advantages of using the derive annotation should be clear here, as the more columns you have, the more code you would have to write.

## <a name="insertable"></a>Insertable

Adding the [`#[derive(Insertable)]`][insertable_doc] trait annotation to your model struct allows records to be inserted into the database via that struct.
You will need `Insertable` when using the `diesel::insert(&my_new_row).into(my_table::table)` API.
You may pass a reference to a single struct to insert a single record. Pass a Vec or a slice to insert multiple records at once.

[insertable_doc]: http://docs.diesel.rs/diesel/prelude/trait.Insertable.html

When implementing `Insertable`, you probably won't be setting the auto-incremented `id` field of the row. 
Usually you will also ignore fields such as `created_at` and `updated_at`.
For this reason, it's not advisable to use `Queryable` and `Insertable` on the same struct due to the field number constraints of `Queryable`.
Create another struct that you may use for database insertions that will have all the fields you would like to set.
This section will not cover nullable fields (we'll cover that in [AsChangeset](#aschangeset)), so we will assume every field must have data in our example.
When making a separate struct for database inserts,
Diesel needs to know the corresponding table name,
so the struct must also be annotated with the `#[table_name="some_table_name"]` attribute.
If your new struct has different field names, each of them may be annotated with `#[column_name(some_column_name)]`.

### Example 

*Note: src/lib.rs with #[macro_use] extern crate diesel_codegen; not shown*

*src/models.rs*
```rust
#[derive(Queryable)]
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
    #[column_name(email)]
    pub electronic_mail: &'a str,
 }
```

*src/main.rs*
```rust
// externs, database connection, and using statement code omitted

fn main() {
  let new_user = NewUser { 
      first_name: "Gordon", 
      last_name: "Freeman", 
      electronic_mail: "gordon.freeman@blackmesa.co" 
  };

  diesel::insert(&new_user).into(users::table)
      .execute(&db_connection)
      .expect("Error inserting row into database");

  let all_users = users.load::<User>(&db_connection)
      .expect("Error loading users");

  println!("User count: {}", all_users.len()); // > User count: 1
}
```

If we try to insert records without deriving `Insertable`, we would get the following error.

```rust

error[E0277]: the trait bound `&diesel_demo::models::NewUser<'_>: diesel::query_builder::insert_statement::IntoInsertStatement<_, diesel::query_builder::insert_statement::Insert>` is not satisfied
  --> src/bin/main.rs:31:30
   |
31 |     diesel::insert(&new_user).into(users::table)
   |                               ^^^^ the trait `diesel::query_builder::insert_statement::IntoInsertStatement<_, diesel::query_builder::insert_statement::Insert>` is not implemented for `&diesel_demo_cli::models::NewUser<'_>`
```

In the above error, the compiler is telling us there is an unsatisfied trait bound from `IntoInsertStatement`.
`diesel::insert()` returns an `DeprecatedIncompleteInsertStatement`,
which implements `into()`.
`into()` specifies the table the `Insertable` data should be passed to.
If your struct isn't `Insertable`, Rust will get confused and ask you implement this trait.

Now we will look at how `Insertable` would be implemented by hand.
This code was generated by the `cargo-expand` crate and slightly modified to reduce namespacing and increase readability.
Required imports are not included for brevity.

```rust
#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

pub struct NewUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: &'a str,
 }

impl <'a, 'insert, DB> Insertable<users::table, DB> for &'insert NewUser<'a>
where DB: Backend,
      (ColumnInsertValue<users::first_name, AsExpr<&'insert &'a str, users::first_name>>,
      ColumnInsertValue<users::last_name, AsExpr<&'insert &'a str, users::last_name>>,
      ColumnInsertValue<users::email, AsExpr<&'insert &'a str, users::email>>): InsertValues<DB>
{
    type Values =
        (ColumnInsertValue<users::first_name, AsExpr<&'insert &'a str, users::first_name>>,
        ColumnInsertValue<users::last_name, AsExpr<&'insert &'a str, users::last_name>>,
        ColumnInsertValue<users::email, AsExpr<&'insert &'a str, users::email>>);

    #[allow(non_shorthand_field_patterns)]
    fn values(self) -> Self::Values {
        use diesel::expression::{AsExpression, Expression};
        use diesel::insertable::ColumnInsertValue;

        let NewUser {
            first_name: ref first_name,
            last_name: ref last_name,
            email: ref email } = *self;


        // Here is the returned tuple of three `ColumnInsertValue`'s
        (
            ColumnInsertValue::Expression(
                  users::first_name,
                  AsExpression::<<users::first_name as Expression>::SqlType>::as_expression(first_name)
            ),

            ColumnInsertValue::Expression(
                users::last_name,
                AsExpression::<<users::last_name as Expression>::SqlType>::as_expression(last_name)
            ),

            ColumnInsertValue::Expression(
                users::last_name,
                AsExpression::<<users::email as Expression>::SqlType>::as_expression(email)
            ),
        )
    }
}

impl <'a: 'insert, 'insert, Op> IntoInsertStatement<users::table, Op> for &'insert NewUser<'a> {
    type InsertStatement = InsertStatement<users::table, Self, Op>;

    fn into_insert_statement(self, target: users::table, operator: Op) -> Self::InsertStatement {
            InsertStatement::no_returning_clause(target, self, operator)
    }
}
```

One interesting piece to note are the values of the tuple for the associated type, `Values`.
An `InsertValue` is a tuple that consists of each column to be inserted as an `ColumnInsertValue`.
The returned value of `values()` has been formatted to illustrate this point.
This implementation is considerably complex and you wouldn't want to be typing this out for each `Insertable` struct.

## <a name="identifiable"></a>Identifiable

The [`#[derive(Identifiable)]`][identifiable_doc] trait is useful when you need to set up database [Associations](#associations) or are using certain Diesel database update features.
Using the `Indentifiable` on a struct means that struct can be identified on a single table in the database. 

[identifiable_doc]: http://docs.diesel.rs/diesel/associations/trait.Identifiable.html

By default, `Identifiable` will assume the primary key is a column named `id`.
If your table's primary key is named differently,
you can annotate the table with the attribute `#[primary_key(some_field_name)` or `#[primary_key(some_field_name, another_field_name)`.
Like `Insertable`, the `Identifiable` trait will assume that the annotated struct will be named in the singular form of the table it corresponds to.
If the name differs you may use the `#[table_name="some_table_name"]` attribute annotation.
Having the `Identifiable` trait provides us the `id()` method on our models,
which returns the value of our record's primary key.

In the following example,
we will look at some of the behavior `Identifiable` provides for us.
We will add the annotation to our `User` struct.
We will then attempt to get the value of the first record's primary key by calling `id()` and also update the first and last name of our user.

### Example 

*Note: src/lib.rs with #[macro_use] extern crate diesel_codegen; not shown*

*src/models.rs*
```rust
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

*src/main.rs*
```rust
// externs, database connection, and using statement code omitted

fn main() {
  let new_user = NewUser { 
      first_name: "Gordon", 
      last_name: "Freeman", 
      email: "gordon.freeman@blackmesa.co" 
  };

  diesel::insert(&new_user).into(users::table)
      .execute(&db_connection)
      .expect("Error inserting row into database");

  let all_users = users.load::<User>(&db_connection)
      .expect("Error loading users");

  println!("User count: {}", all_users.len()); // > User count: 1

  let hero = users.first(&db_connection)::<Users>
      .expect("Error loading first user");

  println!("Our Hero's Id: {}", hero.id()); // > Our Hero's Id: 1

  diesel::update(&hero).set((
      first_name.eq("Alyx"),
      last_name.eq("Vance"),
  )).execute(&db_connection);
  
  let updated_hero = users.first(&db_connection)::<Users>
      .expect("Error loading first user");
  
  println!("Our Hero's update name: {} {}", updated_hero.first_name, updated_hero.last_name); // > Our Hero's updated name: Alyx Vance
}
```

If we were to try and call `id()` and also update our record without implementing `Identifiable`, we would get the following errors.

```rust
error[E0599]: no method named `id` found for type `diesel_demo_cli::models::User` in the current scope
  --> src/bin/main.rs:37:48
   |
37 |     println!("Our Hero's Id: {}", hero.id());
   |                                        ^^ field, not a method
   |
   = help: did you mean to write `hero.id` instead of `hero.id(...)`?
  --> src/bin/main.rs:39:5
   |
39 |     diesel::update(&hero).set((
   |     ^^^^^^^^^^^^^^ the trait `diesel::associations::HasTable` is not implemented for `diesel_demo_cli::models::User`
   |
   = note: required because of the requirements on the impl of `diesel::associations::HasTable` for `&diesel_demo_cli::models::User`
   = note: required because of the requirements on the impl of `diesel::query_builder::IntoUpdateTarget` for `&diesel_demo_cli::models::User`
   = note: required by `diesel::update`

error[E0277]: the trait bound `&diesel_demo_cli::models::User: diesel::Identifiable` is not satisfied
  --> src/bin/main.rs:39:5
   |
39 |     diesel::update(&hero).set((
   |     ^^^^^^^^^^^^^^ the trait `diesel::Identifiable` is not implemented for `&diesel_demo_cli::models::User`
   |
   = note: required because of the requirements on the impl of `diesel::query_builder::IntoUpdateTarget` for `&diesel_demo_cli::models::User`
   = note: required by `diesel::update`
```

From this error,
you can see the compiler assumes we are looking for a field on the struct,
but what we want is just the value of the primary key,
which happens to also be the `id` field.
The compiler is also giving us some hints about trait bounds in each one of those *note* sections.
The first two notes about `HasTable` and `IntoUpdateTarget` 
might be helpful if we refer to the [Diesel API docs][],
but the third error lets us know exactly which trait isn't satisfied! :smile:

[Diesel API docs]: http://docs.diesel.rs/diesel/associations/trait.Identifiable.html

The following code is generated by [`cargo expand`][] to get our example code working again.
[`cargo expand`][] is a crate that expands the macros and derives tat rustc generates for our code.

[`cargo expand`]: https://github.com/dtolnay/cargo-expand

```rust
#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

impl <'ident> Identifiable for &'ident User {
    type Id = (&'ident i32);

    fn id(self) -> Self::Id {
        (&self.id)
    }
}

impl HasTable for User {
    type Table = users::table;

    fn table() -> Self::Table {
        users::table
    }
}
```

## <a name="aschangeset"></a>AsChangeset

For more slightly more complicated database updates,
[`#[derive(AsChangeset)]`][aschangeset_doc] makes your code more ergonomic.
In this section we will change our schema to make our `email` field nullable.
The nullable fields on our model structs will now be of type `Option<T>`.
Usually you do not want to change the primary key of the row or rows that you're updating.
For this reason, we don't want to have `AsChangeset` and `Queryable` annotated on the same struct,
which means we'll need another struct for updating our records.

[aschangeset_doc]: http://docs.diesel.rs/diesel/query_builder/trait.AsChangeset.html

>It's considered good practice in diesel to have one struct per "usage".
E.g., one struct to query users, one to create new users, one to change just the email, etc.
This often corresponds to the way applications access the database.
Don't be afraid to have multiple structs per database table—
they are not like the classes that correspond to tables 1:1 in other ORMs.

Before we dive into an example and possible considerations,
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
Our User struct has an optional `email` field,
which means we get some special behavior from `AsChangeset`.

By default, `AsChangeset` will assume that anytime a field has the value `None`,
we do not want to assign any values to it.
If we truly want to assign a `NULL` value,
we can use the annotation `#[changeset_options(treat_none_as_null="true")]`.
Be careful, as when you are setting your `Option<T>` fields to `None`,
they will be `NULL` in the database instead of ignored.
However, there is a way to have both behaviors on a single struct.
Instead of using the field type: `Option<T>`,
you may use `Option<Option<T>>`.
When updating, a value of `None` will be ignored and a value of `Some(None)` will `NULL` that column.
All three options are shown in the following example code.

### Example 

*Note: src/lib.rs with #[macro_use] extern crate diesel_codegen; not shown*

*src/models.rs*
```rust
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


// This struct will ignore any fields set to None
// and NULL any set to Some(None)
#[derive(AsChangeset)]
#[table_name="users"]
pub struct IgnoreNoneFieldsUpdateUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<Option<&'a str>>,
 }

// This struct will set the column field to NULL if its set to None
#[derive(AsChangeset)]
#[changeset_options(treat_none_as_null="true")]
#[table_name="users"]
pub struct NullNoneFieldsUpdateUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<&'a str>,
 }
```

*src/main.rs*
```rust
// externs, database connection, and using statement code omitted

fn main() {
  // Creating and inserting new user
  let new_user = NewUser { 
      first_name: "Gordon", 
      last_name: "Freeman", 
      email: Some("gordon.freeman@blackmesa.co"),
  };

  diesel::insert(&new_user).into(users::table)
      .execute(&db_connection)
      .expect("Error inserting row into database");

  let all_users = users.load::<User>(&db_connection)
      .expect("Error loading users");

  println!("User count: {}", all_users.len()); // > User count: 1

  // Querying User
  let hero = users.first(&db_connection)::<Users>
      .expect("Error loading first user");

  // Update scenario 1
  let ignore_fields_update = IgnoreNoneFieldsUpdateUser {
      first_name: "Issac",
      last_name: "Kleiner",
      email: None
  }

  diesel::update(&hero).set(&ignore_fields_update)
      .execute(&db_connection);
  
  let updated_hero = users.first(&db_connection)::<Users>
      .expect("Error loading first user");

  println!("Name: {} {} Email: {}", updated_hero.first_name, 
      updated_hero.last_name, 
      updated_hero.email.unwrap()
  );

  // Output
  // > Name: Issac Kleiner Email: gordon.freeman@blackmesa.ca 
 
  // Update scenario 2 
  let null_a_field_update = IgnoreNoneFieldsUpdateUser {
      first_name: "Issac",
      last_name: "Kleiner",
      email: Some(None)
  }

  diesel::update(&hero).set(&null_a_field_update)
      .execute(&db_connection);
  
  let updated_hero = users.first(&db_connection)::<Users>
      .expect("Error loading first user");

  println!("Name: {} {} Email: {}", updated_hero.first_name, 
      updated_hero.last_name, 
      updated_hero.email.unwrap_or("This field is now Nulled".to_string())
  );

  // Output
  // > Name: Issac Kleiner Email: This field is now Nulled

  // Update scenario 3
  let null_fields_update = NullNoneFieldsUpdateUser {
      first_name: "Eli",
      last_name: "Vance",
      email: None
  }

  diesel::update(&hero).set(&null_fields_update)
      .execute(&db_connection);
  
  let updated_hero = users.first(&db_connection)::<Users>
      .expect("Error loading first user");

  println!("Name: {} {} Email: {:?}", updated_hero.first_name, 
      updated_hero.last_name, 
      updated_hero.email.unwrap_or("This is a None value".to_string())
  );

  // Output
  // > Name: Eli Vance Email: This is a None value
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

We hit another trait bound! In order for us to pass in our structs to `set()`,
we need to have the `AsChangeset` traid satisfied.

Below is an example of implementing `AsChangeset` generated by `cargo expand`.

```rust
#[derive(Identifiable, Queryable)]
pub struct User {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
}

struct UpdateUser<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub email: Option<Option<&'a str>>,
}

impl <'a, 'update> AsChangeset for &'update UpdateUser<'a> {
    type Target = users::table;
    type Changeset = (
        Eq<users::first_name, &'update &'a str>,
        Eq<users::last_name, &'update &'a str>,
        Option<Eq<users::email, &'update Option<&'a str>>>
    );
 
    #[allow(non_shorthand_field_patterns)]
    fn as_changeset(self) -> Self::Changeset {
        use ExpressionMethods;
        let UpdateUser {
            first_name: ref first_name,
            last_name: ref last_name,
            email: ref email,
        } = *self;

        (
            users::first_name.eq(first_name),
            users::last_name.eq(last_name),
            email.as_ref().map(|f| users::email.eq(f))
        )
    }
}
```

## <a name="associations"></a>Associations

What good would an ORM be without supporting database relations?
Diesel provides the [`#[derive(Associations)]`][associations_doc] trait to support bi-directional relationships.
In this section will implement `Associations` and access data through via a foreign key.
If you want to use `belongs_to()` and `grouped_by()` methods,
you will need to derive `Associations` or implement the necessary traits by hand.

[associations_doc]: http://docs.diesel.rs/diesel/associations/index.html

While relations are bi-directional,
Diesel relations focus more on the child-to-parent relationship.
This can be seen in how `Associations` are implemented.
The child table will need the `Associations` annotation as well as a `#[belongs_to(ParentStruct)]` attribute annotation.
Both the parent and child struct must have the [Identifiable](#identifiable) trait implemented.
By default, the foreign key column on the child should take the form of `parent_id`.
If there is a custom foreign key,
the `belongs_to` attribute must be written as `#[belongs_to(ParentStruct, foreign_key="my_custom_key")]`.

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

*Note: src/lib.rs with #[macro_use] extern crate diesel_codegen; not shown*

*src/models.rs*
```rust
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
*src/main.rs*
```rust
// externs, database connection, and using statement code omitted

fn main() {
  // Creating and inserting new user
  let new_user = NewUser { 
      first_name: "Issac", 
      last_name: "Kleiner", 
      email: Some("issac.kleiner@blackmesa.co"),
  };

  let issac_kleiner = diesel::insert(&new_user).into(users::table)
      .get_result::<User>(&db_connection)
      .expect("Error inserting row into database");

  // Setting up our new post and pointing it to the 
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
      }
  ];

  // Insert the new posts vector
  diesel::insert(&post_list).into(posts::table)
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

Trying to use the `BelongingToDsl` without deriving `Associations`.

```rust
error[E0599]: no function or associated item named `belonging_to` found for type `diesel_demo::models::Post` in the current scope
  --> src/bin/main.rs:46:24
   |
46 |     let issacs_posts = Post::belonging_to(&issac)
   |                        ^^^^^^^^^^^^^^^^^^
   |
   = note: the method `belonging_to` exists but the following trait bounds were not satisfied:
           `diesel_demo::models::Post : diesel::BelongingToDsl<&[_]>`
           `diesel_demo::models::Post : diesel::associations::HasTable`
           `diesel_demo::models::Post : diesel::associations::BelongsTo<_>`
           `diesel_demo::models::Post : diesel::associations::BelongsTo<diesel_demo::models::User>`
            // ...
```

In this particular case,
we are using the `belongs_to()` method, which requires us to implement the `BelongsTo` trait.
Below is what `cargo expand` generates for `#[derive(Associations)]`.

```rust
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

#[derive(Identifiable, Queryable)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub content: String,
}

impl BelongsTo<User> for Post {
    type ForeignKey = i32;
    type ForeignKeyColumn = posts::user_id;

    fn foreign_key(&self) -> Option<&i32> {
        Some(&self.user_id)
    }

    fn foreign_key_column() -> Self::ForeignKeyColumn {
        posts::user_id
    }
}

#[derive(Insertable)]
#[table_name="posts"]
pub struct NewPost<'a> {
    pub user_id: i32,
    pub title: &'a str,
    pub content: &'a str,
 }
```

## Conclusion

We've now covered the five (`Queryable`, `Insertable`, `Identifiable`, `AsChangeset`, `Associations`) 
derives you should be familiar with when working with Diesel models.
Please check out other official guides, docs, and gitter channel 
for more information on using the Diesel framework.

- [Getting Started](http://diesel.rs/guides/getting-started/)
- [All About Updates](http://diesel.rs/guides/all-about-updates/)
- [Diesel API Docs](http://docs.diesel.rs/diesel/index.html)
- [Gitter channel](https://gitter.im/diesel-rs/diesel)
