
# Diesel Style Guide

## Philosophy

The goal of these style guidelines is to maintain a useful git history,
particularly the ability to get context on a line of code quickly via `git blame`.
As such, much of the code is styled in such a way that lines are unlikely to change
in order to support unrelated changes. 

Note that these are guidelines, not hard rules. Use your best judgement.

**If you find existing code that doesn't already adhere to these guidelines, don't change it simply to adhere to them**

If we start changing code for style reasons only, it completely defeats the purpose of trying to avoid git churn in the first place. Do change code that doesn't adhere to these guidelines if you're already changing that line for other reasons.

## Guidelines

### Brakets on their own lines

If a function, struct, trait, or any other definition spans more than one line, the `{` goes on its own line.

Example:

``` rust
// GOOD
fn foo(some_args: OmgVerboseTypeName, more_args: OmgVerboseTypeTwo)
    -> OurReturnType
{
    // ...
}

// BAD
fn foo(some_args: OmgVerboseTypeName, more_args: OmgVerboseTypeTwo)
    -> OurReturnType {
    // ...
}

// ALSO BAD
fn foo(some_args: OmgVerboseTypeName, more_args: OmgVerboseTypeTwo)
    -> OurReturnType {
        // ...
    }
```

The reasoning for this is simple: While I hate the curly brace being on its own line, I hate having part of the function signature being at the same indentation level as the function body (I need an easily identifiable visual separator), and we need the indentation level of the function body to be independent of the struct definition.

### Do not token align.

Just don't.

It rarely actually improves readability. The next line should start 4 spaces deeper than the previous, at the same indentation level, or 4 spaces shallower. Not 7 because it lets a `:` line up, as we'll have to re-align everything as soon as we add a longer key.


### Always use trailing commas for multiline groupings

Example:

``` rust
// GOOD
let ints = [
    1,
    2,
    3,
]

// BAD
let ints = [
    1,
    2,
    3
]
```

### Keep lines under ~80 characters

You don't need to break it up when it's 81. My rough guideline is \"does this fit in my terminal pane\", which for me ends up being 1 vertical vim split out of 2 on a 15-inch macbook, and 1 vertical vim split out of 3 on a cinema display. This comes out to roughly 80ish characters, but there's no _hard_ character limit. Documentation should have a _hard_ wrap at 80 characters.

### Prefer where clauses to the compact form basically always

Type constraints have a tendency to change more frequently than anything else. This will probably settle down a bit as we approach 1.0, but when a single constraint changes, I don't want to cause churn on the entire signature every time. My general rule of thumb is that you should use a where clause if there is more than one type parameter (even if all but one are unconstrained), or if the type parameter has more than one bound.

### Break things into multi-line when it gets close to being required, not after

If a function signature is pushing it, it's reasonably likely something will change that pushes it over the 
edge. Move to a where clause or multiline signature sooner rather than later.

### State what you mean in where clauses

For example, if we still had `Expression: QueryFragment`, and we had

``` rust
impl<T, U> QueryFragment for Bound<T, U> where
    T: NativeSqlType,
    U: ToSql<T>,
{
    // ...
}
```

then your constraint for `Expression` should show that you're trying to satisfy the constraints for `QueryFragment`, not just repeating them outright.

``` rust
// GOOD
impl<T, U> Expression for Bound<T, U> where
    T: NativeSqlType,
    Bound<T, U>: QueryFragment,
{
    type SqlType = T;
}

// BAD
impl<T, U> Expression for Bound<T, U> where
    T: NativeSqlType,
    U: ToSql<T>
{
    type SqlType = T;
}
```

### How to structure long functions

Here's what a long function signature should be structured like:

``` rust
fn really_long_function_name_omg_we_used_half_our_char_limit<T, U, V>(
    arg1: Type1,
    arg2: Type2,
) -> ReturnType where
    T: Something,
    U: Something,
{
    // body
}
```
