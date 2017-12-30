Composing Applications with Diesel
----------------------------------

One of the main benefits of using a query builder over raw SQL
is that you can pull bits of your query out into functions and reuse them.
In this guide,
we'll look at common patterns for extracting your code into re-usable pieces.
We'll also look at best practices for how to structure your code.

All of our code examples are based on code from crates.io,
a real world application which uses Diesel extensively.
All of our examples will be focused on functions which *return*
queries or pieces of queries.
None of these examples will include a function which takes a database
connection.
We will go into the benefits of this structure at the end of the guide.

crates.io has a `canon_crate_name` SQL function
which is always used when comparing crate names.
Rather than continuously writing
`canon_crate_name(crates::name).eq("some name")`,
we can instead pull this out into a function.

```
use diesel::dsl::Eq;
use diesel::types::Text;

sql_function!(canon_crate_name, CanonCrateName, (x: Text) -> Text);

type WithName<'a> = Eq<canon_crate_name<crates::name>, canon_crate_name<&'a str>>;

fn with_name(name: &str) -> WithName {
    canon_crate_name(crates::name).eq(canon_crate_name(name))
}
```

Now when we want to find a crate by name, we can write
`crates::table.filter(with_name("foo"))` instead.
If we want to accept types other than a string,
we can make the method generic.

```
use diesel::dsl::Eq;
use diesel::types::Text;

sql_function!(canon_crate_name, CanonCrateName, (x: Text) -> Text);

type WithName<T> = Eq<canon_crate_name<crates::name>, canon_crate_name<T>>;

fn with_name<T>(name: T) -> WithName<T>
where
    T: AsExpression<Text>,
{
    canon_crate_name(crates::name).eq(canon_crate_name(name))
}
```

It's up to you whether you make your functions generic,
or only take a single type.
We recommend only making these functions generic if it's actually needed,
since it requires additional bounds in your `where` clause.
The bounds you need might not be clear,
unless you are familiar with Diesel's lower levels.

In these examples,
we are using helper types from `diesel::dsl`
to write the return type explicitly.
Nearly every method in Diesel has a helper type like this.
The first type parameter is the method receiver
(the thing before the `.`).
The remaining type parameters are the arguments to the method.
If we want to avoid writing this return type,
or dynamically return a different expression,
we can box the value instead.

```
use diesel::pg::Pg;
use diesel::types::Text;

sql_function!(canon_crate_name, CanonCrateName, (x: Text) -> Text);

fn with_name<'a, T>(name: T) -> Box<BoxableExpression<crates::table, Pg, SqlType = Bool> + 'a>
where
    T: AsExpression<Text>,
    T::Expression: BoxableExpression<crates::table, Pg>,
{
    canon_crate_name(crates::name).eq(canon_crate_name(name))
}
```

In order to box an expression, Diesel needs to know three things:

- The table you intend to use it on
- The backend you plan to execute it against
- The SQL type it represents

This is all the information Diesel uses to type check your query.
Normally we can get this information from the type,
but since we've erased the type by boxing,
we have to supply it.

The table is used to make sure that you don't try to use `users::name`
on a query against `posts::table`.
We need to know the backend you will execute it on,
so we don't accidentally use a PostgreSQL function on SQLite.
The SQL type is needed so we know what functions this can be passed to.

Boxing an expression also implies that it has no aggregate functions.
You cannot box an aggregate expression in Diesel.
As of Diesel 1.0, a boxed expression can only be used with *exactly* the from
clause given.
You cannot use a boxed expression for `crates::table` with an inner join to
another table.

In addition to extracting expressions,
you can also pull out entire queries into functions.
Going back to crates.io,
the `Crate` struct doesn't use every column from the `crates` table.
Because we almost always select a subset of these columns,
we have an `all` function which selects the columns we need.

```
use diesel::dsl::Select;

type AllColumns = (
    crates::id,
    crates::name,
    crates::updated_at,
    crates::created_at,
);

const ALL_COLUMNS = (
    crates::id,
    crates::name,
    crates::updated_at,
    crates::created_at,
);

type All = Select<crates::table, AllColumns>;

impl Crate {
    pub fn all() -> All {
        crates::table.select(ALL_COLUMNS)
    }
}
```

We also frequently found ourselves writing
`Crate::all().filter(with_name(crate_name))`.
We can pull that into a function as well.

```
use diesel::dsl::Filter;

type ByName<T> = Filter<All, WithName<T>>;

impl Crate {
    fn by_name<T>(name: T) -> ByName<T> {
        Self::all().filter(with_name(name))
    }
}
```

And just like with expressions,
if we don't want to write the return types,
or we want to dynamically construct the query differently,
we can box the whole query.

```rust
use diesel::expression::{Expression, AsExpression};
use diesel::pg::Pg;
use diesel::types::Text;

type SqlType = <AllColumns as Expression>::SqlType;
type BoxedQuery<'a> = crates::BoxedQuery<'a, Pg, SqlType>;

impl Crate {
    fn all() -> BoxedQuery<'static> {
        crates::table().select(ALL_COLUMNS).into_boxed()
    }

    fn by_name<'a, T>(name: T) -> BoxedQuery<'a>
    where
        T: AsExpression<Text>,
        T::Expression: BoxableExpression<crates::table, Pg>,
    {
        Self::all().filter(by_name(name))
    }
}
```

Once again, we have to give Diesel some information to box the query:

- The SQL type of the `SELECT` clause
- The `FROM` clause
- The backend you are going to execute it against

The SQL type is needed so we can determine what structs can be
deserialized from this query.
The `FROM` clause is needed so we can validate the arguments
to future calls to `filter` and other query builder methods.
The backend is needed to ensure you don't accidentally use a
PostgreSQL function on SQLite.

Note that in all of our examples,
we are writing functions which *return* queries or expressions.
None of these functions execute the query.
In general you should always prefer functions which return queries,
and avoid functions which take a connection as an argument.
This allows you to re-use and compose your queries.

For example, if we had written our `by_name` function like this:

```
impl Crate {
    fn by_name(name: &str, conn: &PgConnection) -> QueryResult<Self> {
        Self::all()
            .filter(with_name(name))
            .first(conn)
    }
}
```

Then we would never be able to use this query in another context,
or modify it further.
By writing the function as one that returns a query,
rather than executing it,
we can do things like use it as a subselect.

```
let version_id = versions
    .select(id)
    .filter(crate_id.eq_any(Crate::by_name(crate_name).select(crates::id)))
    .filter(num.eq(version))
    .first(&*conn)?;
```

Or use it to do things like get all of its downloads:

```
let recent_downloads = Crate::by_name(crate_name)
    .inner_join(crate_downloads::table)
    .filter(CrateDownload::is_recent())
    .select(sum(crate_downloads::downloads))
    .get_result(&*conn)?;
```
