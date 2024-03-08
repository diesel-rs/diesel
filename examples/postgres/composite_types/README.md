# diesel-postgres-composite-type
This repository contains a series of examples to demonstrate how [composite types](https://www.postgresql.org/docs/current/rowtypes.html) in PostgreSQL can
be used in the diesel query builder.

This manual assumes you're familiar with [PostgreSQL](https://www.postgresql.org/docs/)
and [Rust](https://www.rust-lang.org/learn). As I struggled to understand
how you can use [Diesel](https://diesel.rs) with PostgreSQL's composite types,
I have written a series of examples which stepwise introduces the required
methods and traits to deal with this.

What will be discussed?
* Importing data from postgreSQL to Rust [anonymously](README.md#from-pgsql-to-rust-anonymously-coordinates).
* Importing data from postgreSQL to Rust [using named fields](README.md#from-pgsql-to-rust-with-type-binding-colors).
* Exporting data from Rust to PostgreSQL (TODO)


# From pgSQL to Rust anonymously: coordinates.
Let's start with a simple table containing an unique identifier and two columns
with integers. After downloading the repository and running `diesel migration run`
you should be able to see the following, using e.g. the terminal `psql`:

```sql
composite_type_db=# SELECT * FROM coordinates;
 coord_id | xcoord | imaginairy_part
----------+-----------+-----------------
        1 |         1 |               0
        2 |         0 |               1
        3 |         1 |               1
        4 |         3 |               4
```
### Get the used types from the column definition
The typical working flow when using Diesel is to automatically generate [schema.rs](./src/schema.rs) which
provides us with the type information of the columns which are present in
our coordinates table. Also, an SQL function, [distance_from_origin()](./migrations/2023-10-23-111951_composite2rust_coordinates/up.sql),
is defined. We need to explain this to the Rust compiler using the [define_sql_function!](https://docs.rs/diesel/latest/diesel/expression/functions/macro.define_sql_function.html)
macro like this:
```rust
define_sql_function!(fn distance_from_origin(re: Integer,im: Integer) -> Float);
```
Keep in mind that we specify [only postgreSQL types](https://docs.rs/diesel/latest/diesel/sql_types/index.html)
as the input parameters and return value(s) of this function. If the columns
names are also *in scope* then we can write in Rust:

```rust
let results: Vec<(i32, f32)> = coordinates
    .select((coord_id, distance_from_origin(xcoord, ycoord)))
    .load(connection)?;
```
So we expect a vector of a 2-tuple, or ordered pair of the Rust types ```i32```
and ```f32```. Mind that the float type is not present in the table but is
specified in by the SQL function in the database and also in the macro `define_sql_function!`
definition above. Of course we can expand this to very long tuples, but that
will become error prone as we have to specify the sequence of type correctly
every function call. Try out the [first example](./examples/composite2rust_coordinates) with:

```sh
cargo run --example composite2rust_coordinates
```

### Define an alias type
To avoid errors we could define an [alias type](https://doc.rust-lang.org/stable/std/keyword.type.html)
once and use this in the various calls of our function.
```rust
type Distance = (i32, f32);
```
The re-written function call will then look like:
```rust
let results: Vec<Distance> = coordinates
    .select((coord_id, distance_from_origin(xcoord, ycoord)))
    .load(connection)?;
```

### Reducing the output to a single value instead of an array
The default output of a query to the database is a table with results.
However, frequenty we may only expect a single answer, especially if a function
has defined several **OUT**-*put* parameters instead of returning a table, like the
created SQL function ```shortest_distance()```.
To avoid the destructering of the vector, when a vector is not needed, we
can use the ```get_result()``` instead of the ```load()``` function call
with our specified type:

```rust
let result: Distance = select(shortest_distance())
    .get_result(connection)?;
```

### Creating a type in PostgreSQL world
Using a tuple only enforces the correct number of return values and their basic type.
If multiple values of the same type are returned, they can easily be mixed-up without
any warning. Therefore, to improve readability of the database SQL functions it makes
sense to introduce new types, like for example this one:
```sql
CREATE TYPE my_comp_type AS (coord_id INTEGER, distance FLOAT4);
```
If we specified a database function ```longest_distance()``` we can simply
use that now on the Rust side with:

```rust
let result: Distance = select(longest_distance())
    .get_result(connection)?;
```
So, although we specified a new type in the database, we **don't need** to
specify it on the Rust side too. If we never make errors, that would be a
possible solution. However, like unsafe Rust, this is not recommended. Why
build in possible pitfalls if we can avoid them?

# From pgSQL to Rust with type binding: colors.
In the [second example](./examples/composite2rust_colors.rs) we want to convert any RGB value, consisting of three integer values, to a new type which expresses the reflected light and suggests a name for this reflection:
```sql
CREATE TYPE gray_type AS (intensity FLOAT4, suggestion TEXT);
```
This new type will be used in two exactly the same SQL functions `color2grey()` and `color2gray()` of which input and return value are specified like:
```sql
CREATE FUNCTION color2grey(
    red    INTEGER,
    green  INTEGER,
    blue   INTEGER
) RETURNS gray_type AS
$$
...
```
You can run the example with the following command:
```sh
cargo run --example composite2rust_colors
```
On the Rust side, we define the interpretation of both functions differently, the first one using a tuple similar to the coordinates example, the second one using a _locally_ defined Rust type for interpreting a tuple: notice the **Pg**-prefix of `PgGrayType`.

```rust
define_sql_function!(fn color2grey(r: Integer, g: Integer,b: Integer) -> Record<(Float,Text)>);
define_sql_function!(fn color2gray(r: Integer, g: Integer,b: Integer) -> PgGrayType);
```
As this only creates a type with anonymous fields, which can be addressed by their field number **object.0**, **object.1** etc., it would be more convenient to attach names to the fields. Therefore we need to define a type with our intended field names, which we can use _globally_ (or at least outside the database related code space):
```rust
#[derive(Debug, FromSqlRow)]
pub struct GrayType {
    pub intensity: f32,
    pub suggestion: String,
}
```
The derived [FromSqlRow](https://docs.rs/diesel/latest/diesel/deserialize/trait.FromSqlRow.html) trait explains Diesel it is allowed to convert a tuple to this new type. We only need a implementation on _how_ to do that for a PostgreSQL backend:
```rust
impl FromSql<PgGrayType, Pg> for GrayType {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        let (intensity, suggestion) = FromSql::<PgGrayType, Pg>::from_sql(bytes)?;
        Ok(GrayType {
            intensity,
            suggestion,
        })
    }
}
```
Although this seems trivial for this example, it also allows the posssibility to add some more checks or modifications on the imported data: we could for example limit the values of intensity between 0 and 100%.


Did you read the [License](./LICENSE)?








# Miscellaneous, Set-up etc.
Switch to user postgres with the following terminal command:
```bash
    su - postgres
    psql
```
In this psql terminal do:
```sql
CREATE DATABASE composite_type_db ENCODING 'UTF8'  LC_COLLATE='C'  LC_CTYPE='C'  template=template0 OWNER postgres;
```
this should reply with:
```
CREATE DATABASE
```
You can verify the list of present databases with typing `\l` and then exit with `\q`

    echo DATABASE_URL=postgres://username:password@localhost/diesel_demo > .env

Create it with the diesel command (will create database if it didn't exist, but with your locale  settings.):
    diesel setup

composite_type_db
