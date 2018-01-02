Extending Diesel's Query Builder
================================

Diesel provides a lot of capabilities out of the box.
However, it doesn't necessarily provide everything your app may want to use.
One of Diesel's greatest strengths,
is that it can be extended to add new functionality.

In this guide we'll look at several ways to hook into Diesel's query builder,
both to add new capabilities,
and to introduce new abstractions.

This guide is only going to cover extending the query builder.
How to add support for new SQL types will be covered in a future guide.

`sql_function!`
---------------

The easiest and most common way to extend Diesel's query builder
is by declaring a new SQL function.
This can be used for functions defined by your database,
or for built-in functions that Diesel doesn't support out of the box.

Functions in SQL often have multiple signatures,
making them difficult or impossible to represent in Rust.
Because of this, Diesel only provides support for a small number
of built-in SQL functions.
Consider `COALESCE`.
This function can take any number of arguments,
and its return type changes based on whether any arguments are `NOT NULL`.
While we can't easily represent that in Rust,
we can use `sql_function!` to declare it with the exact signature we're using.

```
use diesel::types::{Nullable, Text};
sql_function!(coalesce, Coalesce, (x: Nullable<Text>, y: Text) -> Text);

users.select(coalesce(hair_color, "blue"))
```

As this example shows,
`sql_function!` converts it's argument like other parts of the query builder.
This means that the generated function can take both Diesel expressions,
and Rust values to be sent with the query.

The macro takes three arguments:

- A function name
- A type name
- A type signature

The type signature uses the same syntax as a normal Rust function.
However, the types given are SQL types,
not concrete Rust types.
This is what allows us to pass both columns and Rust strings.
If we defined this function manually, it would look like this:

```
fn coalesce<X, Y>(x: X, y: Y) -> Coalesce<X::Expression, Y::Expression>
where
    X: AsExpression<Nullable<Text>>,
    Y: AsExpression<Text>,
```

The type name given as the second argument is almost never used.
Instead, a helper type is generated with the same name as the function.
This helper type handle's Diesel's argument conversion.
This lets us write `coalesce<hair_color, &str>`
instead of `Coalsece<hair_color, Bound<Text, &str>>`.

Custom SQL
----------

Often times it's useful to encapsulate a common SQL pattern.
For example, if you're doing pagination on your queries,
PostgreSQL is capable of loading the total count in a single query.
The query you would want to execute would look like this:

```
SELECT *, COUNT(*) OVER () FROM (subselect t) LIMIT $1 OFFSET $1
```

However, as of version 1.0,
Diesel doesn't support window functions, or selecting from a subselect.
Even if Diesel's query builder supported those things,
this is a case that is easier to reason about in terms of the SQL we want to
generate.

Let's look at how we would go about adding a `paginate` method to Diesel's query
builder, to generate that query.
Let's assume for the time being that we have a struct `Paginated<T>` already.
We'll look at the specifics of this struct shortly.

If you are creating a struct where you want to manually define the SQL,
you will need to implement a trait called `QueryFragment`.
The implementation will look like this:

```
impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.limit())?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset())?;
        Ok(())
    }
}
```

For details on what each method does,
see the documentation for [`AstPass`].
One important question to ask whenever you implement `QueryBuilder`
is whether you are generating a query that is safe to cache.
The way to answer this question is by asking
"does this struct generate an unlimited number of potential SQL queries"?
Typically that is only the case if the body of `walk_ast` contains a for loop.
If your query is not safe to cache, you *must* call
`out.unsafe_to_cache_prepared`.

Whenever you implement `QueryFragment`, you also need to implement `QueryId`.
We can use the [`impl_query_id!`] macro for this.
Since this struct represents a full query which can be executed,
we will implement [`RunQueryDsl`] which adds methods like [`execute`] and [`load`].
Since this query has a return type,
we'll implement [`Query`] which states the return type as well.

[`AstPass`]: //docs.diesel.rs/diesel/query_builder/struct.AstPass.html
[`impl_query_id!`]: //docs.diesel.rs/diesel/macro.impl_query_id.html
[`RunQueryDsl`]: //docs.diesel.rs/diesel/query_dsl/trait.RunQueryDsl.html
[`execute`]: //docs.diesel.rs/diesel/query_dsl/trait.RunQueryDsl.html#method.execute
[`load`]: //docs.diesel.rs/diesel/query_dsl/trait.RunQueryDsl.html#method.load
[`Query`]: //docs.diesel.rs/diesel/query_builder/trait.Query.html

```
impl_query_id!(Paginated<T>);

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<PgConnection> for Paginated<T> {}
```

Now that we've implemented all of these things,
let's look at how we would go about constructing this.
We'll want to add a `paginate` method to all Diesel queries,
which specifies which page we're on,
as well as a `per_page` method which specifies the number of elements per page.

Once again, we can extend existing types with a trait.

```
pub trait Paginate: AsQuery + Sized {
    fn paginate(self, page: i64) -> Paginated<Self::Query> {
        Paginated {
            query: self.as_query(),
            page,
            per_page: DEFAULT_PER_PAGE,
         }
    }
}

impl<T: AsQuery> Paginate for T {}

const DEFAULT_PER_PAGE: i64 = 10;

pub struct Paginated<T> {
    query: T,
    page: i64,
    per_page: i64,
}

impl Paginated<T> {
    pub fn per_page(self, per_page: i64) -> Self {
        Paginated { per_page, ..self }
    }
}
```

Now we can get the third page of a query with 25 elements per page like this:

```
users::table
    .paginate(3)
    .per_page(25)
```

With this code,
we could load any query into a `Vec<(T, i64)>`,
but we can do better.
When doing pagination,
you usually want the records and the total number of pages.
We can write that method.

```
impl<T> Paginated<T> {
    fn load_and_count_pages<U>(self, conn: &PgConnection) -> QueryResult<(Vec<U>, i64)
    where
        Self: LoadQuery<PgConnection, (U, i64)>,
    {
        let per_page = self.per_page;
        let results = self.load::<(U, i64)>(conn)?;
        let total = results.get(0).map(|(_, total) total|).unwrap_or(0);
        let records = results.into_iter().map(|(record, _)| record).collect();
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Ok((records, total_pages))
    }
}
```

This is one of the rare cases where we want to define a function that takes a
connection.
One benefit of defining the function this way
is that if we wanted to support backends other than PostgreSQL,
we could have this function execute two queries.

You can find the full code for this example in [the "advanced blog" example]

Custom Operators
----------------

If you're adding support for a new type to Diesel,
or working with a type that has incomplete support,
you may wish to add support for the operators associated with that type.
The term operator refers to anything that uses one of these syntaxes:

- Infix (e.g. `left OP right`)
- Prefix (e.g. `OP expr`)
- Postfix (e.g. `expr OP`)

Diesel provides helper macros for defining each of these kinds of operators.
In fact, Diesel uses these macros to declare nearly all of the operators
supported by the main crate.
The macros are
[`diesel_infix_operator!`], [`diesel_postfix_operator!`] and
[`diesel_prefix_operator!`].

[`diesel_infix_operator!`]: (//docs.diesel.rs/diesel/macro.diesel_infix_operator.html)
[`diesel_postfix_operator!`]: (//docs.diesel.rs/diesel/macro.diesel_postfix_operator.html)
[`diesel_prefix_operator!`]: (//docs.diesel.rs/diesel/macro.diesel_prefix_operator.html)

All of these macros have the same signature.
They take between two and four arguments.

The first is the name of the struct you want to represent this operator.

The second is the actual SQL for this operator.

The third argument is optional, and is the SQL type of the operator.
If the SQL type is not specified, it will default to `Bool`.
You can also pass the "magic" type `ReturnBasedOnArgs`,
which will cause the SQL type to be the same as the type of its arguments.
Diesel uses this to make the string concatenation operator `||`
return `Nullable<Text>` if the arguments are nullable,
or `Text` if they are not null.

The fourth argument (or third if you didn't specify the SQL type)
is the backend this operator is used for.
If you don't specify a backend,
the operator can be used on all backends.

Let's look at some example usage from Diesel:

```
// A simple operator. It returns `Bool` and works on all backends.
diesel_infix_operator!(Eq, " = ");

// Here we've specified the SQL type.
// Since this operator is only used for ordering, and we don't want it used
// elsewhere, we've made it `()` which is normally useless.
diesel_postfix_operator!(Asc, " ASC", ());

// Concat uses the magic `ReturnBasedOnArgs` return type
// so it can work with both `Text` and `Nullable<Text>`.
diesel_infix_operator!(Concat, " || ", ReturnBasedOnArgs);

// This operator is PG specific, so we specify the backend
diesel_infix_operator!(IsDistinctFrom, " IS DISTINCT FROM ", backend: Pg);

// This operator is PG specific, and we are also specifying the SQL type.
diesel_postfix_operator!(NullsFirst, " NULLS FIRST", (), backend: Pg);
```

Diesel provides a proof-of-concept crate showing how to add new SQL types called
`diesel_full_text_search`.
These are the operators as they are defined in that crate.
You'll notice all of the operators specify the backend,
and many of them specify the return type.

```
diesel_infix_operator!(Matches, " @@ ", backend: Pg);
diesel_infix_operator!(Concat, " || ", TsVector, backend: Pg);
diesel_infix_operator!(And, " && ", TsQuery, backend: Pg);
diesel_infix_operator!(Or, " || ", TsQuery, backend: Pg);
diesel_infix_operator!(Contains, " @> ", backend: Pg);
diesel_infix_operator!(ContainedBy, " <@ ", backend: Pg);
```

However, just declaring the operator by itself isn't very useful.
This creates the types required by Diesel's query builder,
but doesn't provide anything to help use the operator in real code.
The structs created by these macros will have a `new` method,
but that's not typically how you work with Diesel's query builder.

- Infix operators are usually methods on the left hand side.
- Postfix operators are usually methods on the expression.
- Prefix operators are usually bare functions.

For operators that you create with methods,
you would typically create a trait for this.
For example, here's how the `.eq` method gets defined by Diesel.

```
pub trait ExpressionMethods: Expression + Sized {
    fn eq<T: AsExpression<Self::SqlType>>(self, rhs: T) -> Eq<Self, T::Expression> {
        Eq::new(self, other.as_expression())
    }
}

impl<T: Expression> ExpressionMethods for T {}
```

It's important to note that these methods are where you should put any type
constraints.
The structs defined by `diesel_*_operator!` don't know or care about what the
types of the arguments should be.
The `=` operator requires that both sides be of the same type,
so we represent that in the type of `ExpressionMethods::eq`.

You'll also notice that our argument is
`AsExpression<Self::SqlType>`,
not `Expression<SqlType = Self::SqlType>`.
This allows Rust values to be passed as well as Diesel expressions.
For example, we can do `text_col.eq(other_text_col)`,
or `text_col.eq("Some Rust string")`.

If the operator is specific to only one SQL type,
we can represent that in our trait.

```
pub trait BoolExpressionMethods: Expression<SqlType = Bool> + Sized {
    fn and<T: AsExpression<Bool>>(self, other: T) -> And<Self, T::Expression> {
        And::new(self, other.as_expression())
    }
}

impl<T: Expression<SqlType = Bool>> BoolExpressionMethods for T {}
```

Prefix operators are usually defined as bare functions.
The code is very similar, but without the trait.
Here's now `not` is defined in Diesel.

```
pub fn not<T: AsExpression<Bool>>(expr: T) -> Not<Grouped<T::Expression>> {
    super::operators::Not::new(Grouped(expr.as_expression()))
}
```

In this case we're using `Grouped` to add parenthesis around our argument.
This ensures that the operator precedence in SQL matches what's expected.
For example, if we would expect `not(true.and(false))` to return `true`.
However, `SELECT NOT TRUE AND FALSE` returns `FALSE`.
Diesel does the same thing with `.or`.

It's also a best practice to expose a "helper type" for your method,
which does the same type conversion as the method itself.
Nobody wants to write `Eq<text_col, <&str as AsExpression<Text>>::Expression>`.
Instead, we provide a type that lets you write `Eq<text_col, &str>`.

```
pub type Eq<Lhs, Rhs> = super::operators::Eq<Lhs, AsExpr<Rhs, Lhs>>`
```

For defining these types,
you'll usually want to make use of [`SqlTypeOf`], [`AsExpr`], and [`AsExprOf`].

[`SqlTypeOf`]: //docs.diesel.rs/diesel/helper_types/type.SqlTypeOf.html
[`AsExpr`]: //docs.diesel.rs/diesel/helper_types/type.AsExpr.html
[`AsExprOf`]: //docs.diesel.rs/diesel/helper_types/type.AsExprOf.html
